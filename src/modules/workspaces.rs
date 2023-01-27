use crate::clients::compositor::{Compositor, WorkspaceUpdate};
use crate::config::CommonConfig;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::{send_async, try_send};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::Button;
use serde::Deserialize;
use std::collections::HashMap;
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

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

/// Creates a button from a workspace
fn create_button(
    name: &str,
    focused: bool,
    name_map: &HashMap<String, String>,
    tx: &Sender<String>,
) -> Button {
    let button = Button::builder()
        .label(name_map.get(name).map_or(name, String::as_str))
        .build();

    let style_context = button.style_context();
    style_context.add_class("item");

    if focused {
        style_context.add_class("focused");
    }

    {
        let tx = tx.clone();
        let name = name.to_string();
        button.connect_clicked(move |_item| {
            try_send!(tx, name.clone());
        });
    }

    button
}

impl Module<gtk::Box> for WorkspacesModule {
    type SendMessage = WorkspaceUpdate;
    type ReceiveMessage = String;

    fn name() -> &'static str {
        "workspaces"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        // Subscribe & send events
        spawn(async move {
            let mut srx = {
                let client =
                    Compositor::get_workspace_client().expect("Failed to get workspace client");
                client.subscribe_workspace_change()
            };

            trace!("Set up Sway workspace subscription");

            while let Ok(payload) = srx.recv().await {
                send_async!(tx, ModuleUpdateEvent::Update(payload));
            }
        });

        // Change workspace focus
        spawn(async move {
            trace!("Setting up UI event handler");

            while let Some(name) = rx.recv().await {
                let client =
                    Compositor::get_workspace_client().expect("Failed to get workspace client");
                client.focus(name)?;
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
        let container = gtk::Box::new(info.bar_position.get_orientation(), 0);

        let name_map = self.name_map.unwrap_or_default();

        let mut button_map: HashMap<String, Button> = HashMap::new();

        {
            let container = container.clone();
            let output_name = info.output_name.to_string();

            // keep track of whether init event has fired previously
            // since it fires for every workspace subscriber
            let mut has_initialized = false;

            context.widget_rx.attach(None, move |event| {
                match event {
                    WorkspaceUpdate::Init(workspaces) => {
                        if !has_initialized {
                            trace!("Creating workspace buttons");
                            for workspace in workspaces {
                                if self.all_monitors || workspace.monitor == output_name {
                                    let item = create_button(
                                        &workspace.name,
                                        workspace.focused,
                                        &name_map,
                                        &context.controller_tx,
                                    );
                                    container.add(&item);
                                    button_map.insert(workspace.name, item);
                                }
                            }
                            container.show_all();
                            has_initialized = true;
                        }
                    }
                    WorkspaceUpdate::Focus { old, new } => {
                        let old = button_map.get(&old.name);
                        if let Some(old) = old {
                            old.style_context().remove_class("focused");
                        }

                        let new = button_map.get(&new.name);
                        if let Some(new) = new {
                            new.style_context().add_class("focused");
                        }
                    }
                    WorkspaceUpdate::Add(workspace) => {
                        if self.all_monitors || workspace.monitor == output_name {
                            let name = workspace.name;
                            let item = create_button(
                                &name,
                                workspace.focused,
                                &name_map,
                                &context.controller_tx,
                            );

                            item.show();
                            container.add(&item);

                            if !name.is_empty() {
                                button_map.insert(name, item);
                            }
                        }
                    }
                    WorkspaceUpdate::Move(workspace) => {
                        if !self.all_monitors {
                            if workspace.monitor == output_name {
                                let name = workspace.name;
                                let item = create_button(
                                    &name,
                                    workspace.focused,
                                    &name_map,
                                    &context.controller_tx,
                                );

                                item.show();
                                container.add(&item);

                                if !name.is_empty() {
                                    button_map.insert(name, item);
                                }
                            } else if let Some(item) = button_map.get(&workspace.name) {
                                container.remove(item);
                            }
                        }
                    }
                    WorkspaceUpdate::Remove(workspace) => {
                        let button = button_map.get(&workspace.name);
                        if let Some(item) = button {
                            container.remove(item);
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
