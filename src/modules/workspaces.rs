use crate::await_sync;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::sway::{get_client, get_sub_client};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, Orientation};
use serde::Deserialize;
use std::collections::HashMap;
use swayipc_async::{Workspace, WorkspaceChange, WorkspaceEvent};
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::trace;

#[derive(Debug, Deserialize, Clone)]
pub struct WorkspacesModule {
    /// Map of actual workspace names to custom names.
    name_map: Option<HashMap<String, String>>,

    /// Whether to display buttons for all monitors.
    #[serde(default = "crate::config::default_false")]
    all_monitors: bool,
}

#[derive(Clone, Debug)]
pub enum WorkspaceUpdate {
    Init(Vec<Workspace>),
    Update(Box<WorkspaceEvent>),
}

/// Creates a button from a workspace
fn create_button(
    workspace: &Workspace,
    name_map: &HashMap<String, String>,
    tx: &Sender<String>,
) -> Button {
    let button = Button::builder()
        .label(
            name_map
                .get(workspace.name.as_str())
                .unwrap_or(&workspace.name),
        )
        .build();

    let style_context = button.style_context();
    style_context.add_class("item");

    if workspace.focused {
        style_context.add_class("focused");
    }

    {
        let tx = tx.clone();
        let name = workspace.name.clone();
        button.connect_clicked(move |_item| {
            tx.try_send(name.clone())
                .expect("Failed to send workspace click event");
        });
    }

    button
}

impl Module<gtk::Box> for WorkspacesModule {
    type SendMessage = WorkspaceUpdate;
    type ReceiveMessage = String;

    fn spawn_controller(
        &self,
        info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let workspaces = {
            trace!("Getting current workspaces");
            let workspaces = await_sync(async {
                let sway = get_client().await;
                let mut sway = sway.lock().await;
                sway.get_workspaces().await
            })?;

            if self.all_monitors {
                workspaces
            } else {
                trace!("Filtering workspaces to current monitor only");
                workspaces
                    .into_iter()
                    .filter(|workspace| workspace.output == info.output_name)
                    .collect()
            }
        };

        tx.try_send(ModuleUpdateEvent::Update(WorkspaceUpdate::Init(workspaces)))
            .expect("Failed to send initial workspace list");

        // Subscribe & send events
        spawn(async move {
            let mut srx = {
                let sway = get_sub_client();
                sway.subscribe_workspace()
            };

            trace!("Set up Sway workspace subscription");

            while let Ok(payload) = srx.recv().await {
                tx.send(ModuleUpdateEvent::Update(WorkspaceUpdate::Update(payload)))
                    .await
                    .expect("Failed to send workspace update");
            }
        });

        // Change workspace focus
        spawn(async move {
            trace!("Setting up UI event handler");
            let sway = get_client().await;
            while let Some(name) = rx.recv().await {
                let mut sway = sway.lock().await;
                sway.run_command(format!("workspace {}", name)).await?;
            }

            Ok::<(), Report>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<gtk::Box>> {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let name_map = self.name_map.unwrap_or_default();

        let mut button_map: HashMap<String, Button> = HashMap::new();

        {
            let container = container.clone();
            let output_name = info.output_name.to_string();

            context.widget_rx.attach(None, move |event| {
                match event {
                    WorkspaceUpdate::Init(workspaces) => {
                        trace!("Creating workspace buttons");
                        for workspace in workspaces {
                            let item = create_button(&workspace, &name_map, &context.controller_tx);
                            container.add(&item);
                            button_map.insert(workspace.name, item);
                        }
                        container.show_all();
                    }
                    WorkspaceUpdate::Update(event) if event.change == WorkspaceChange::Focus => {
                        let old = event.old.and_then(|old| button_map.get(&old.name));
                        if let Some(old) = old {
                            old.style_context().remove_class("focused");
                        }

                        let new = event.current.and_then(|new| button_map.get(&new.name));
                        if let Some(new) = new {
                            new.style_context().add_class("focused");
                        }
                    }
                    WorkspaceUpdate::Update(event) if event.change == WorkspaceChange::Init => {
                        if let Some(workspace) = event.current {
                            if self.all_monitors || workspace.output == output_name {
                                let item =
                                    create_button(&workspace, &name_map, &context.controller_tx);

                                item.show();
                                container.add(&item);
                                button_map.insert(workspace.name, item);
                            }
                        }
                    }
                    WorkspaceUpdate::Update(event) if event.change == WorkspaceChange::Move => {
                        if let Some(workspace) = event.current {
                            if !self.all_monitors {
                                if workspace.output == output_name {
                                    let item = create_button(
                                        &workspace,
                                        &name_map,
                                        &context.controller_tx,
                                    );

                                    item.show();
                                    container.add(&item);
                                    button_map.insert(workspace.name, item);
                                } else if let Some(item) = button_map.get(&workspace.name) {
                                    container.remove(item);
                                }
                            }
                        }
                    }
                    WorkspaceUpdate::Update(event) if event.change == WorkspaceChange::Empty => {
                        if let Some(workspace) = event.current {
                            if let Some(item) = button_map.get(&workspace.name) {
                                container.remove(item);
                            }
                        }
                    }
                    WorkspaceUpdate::Update(_) => {}
                };

                Continue(true)
            });
        }

        Ok(ModuleWidget {
            widget: container,
            popup: None,
        })
    }
}
