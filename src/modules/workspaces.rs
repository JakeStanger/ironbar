use crate::clients::compositor::{Compositor, WorkspaceUpdate};
use crate::config::CommonConfig;
use crate::image::new_icon_button;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::{send_async, try_send};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, IconTheme};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::trace;

#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// Shows workspaces in the order they're added
    Added,
    /// Shows workspaces in numeric order.
    /// Named workspaces are added to the end in alphabetical order.
    Alphanumeric,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Alphanumeric
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkspacesModule {
    /// Map of actual workspace names to custom names.
    name_map: Option<HashMap<String, String>>,

    /// Whether to display buttons for all monitors.
    #[serde(default = "crate::config::default_false")]
    all_monitors: bool,

    #[serde(default)]
    sort: SortOrder,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

/// Creates a button from a workspace
fn create_button(
    name: &str,
    focused: bool,
    name_map: &HashMap<String, String>,
    icon_theme: &IconTheme,
    tx: &Sender<String>,
) -> Button {
    let label = name_map.get(name).map_or(name, String::as_str);

    let button = new_icon_button(label, icon_theme, 32);
    button.set_widget_name(name);

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

fn reorder_workspaces(container: &gtk::Box) {
    let mut buttons = container
        .children()
        .into_iter()
        .map(|child| (child.widget_name().to_string(), child))
        .collect::<Vec<_>>();

    buttons.sort_by(|(label_a, _), (label_b, _a)| {
        match (label_a.parse::<i32>(), label_b.parse::<i32>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => label_a.cmp(label_b),
        }
    });

    for (i, (_, button)) in buttons.into_iter().enumerate() {
        container.reorder_child(&button, i as i32);
    }
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
            let icon_theme = info.icon_theme.clone();

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
                                        &icon_theme,
                                        &context.controller_tx,
                                    );
                                    container.add(&item);

                                    button_map.insert(workspace.name, item);
                                }
                            }

                            if self.sort == SortOrder::Alphanumeric {
                                reorder_workspaces(&container);
                            }

                            container.show_all();
                            has_initialized = true;
                        }
                    }
                    WorkspaceUpdate::Focus { old, new } => {
                        let old = button_map.get(&old);
                        if let Some(old) = old {
                            old.style_context().remove_class("focused");
                        }

                        let new = button_map.get(&new);
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
                                &icon_theme,
                                &context.controller_tx,
                            );

                            container.add(&item);
                            if self.sort == SortOrder::Alphanumeric {
                                reorder_workspaces(&container);
                            }

                            item.show();

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
                                    &icon_theme,
                                    &context.controller_tx,
                                );

                                container.add(&item);

                                if self.sort == SortOrder::Alphanumeric {
                                    reorder_workspaces(&container);
                                }

                                item.show();

                                if !name.is_empty() {
                                    button_map.insert(name, item);
                                }
                            } else if let Some(item) = button_map.get(&workspace.name) {
                                container.remove(item);
                            }
                        }
                    }
                    WorkspaceUpdate::Remove(workspace) => {
                        let button = button_map.get(&workspace);
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
