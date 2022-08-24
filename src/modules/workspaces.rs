use crate::modules::{Module, ModuleInfo};
use crate::sway::{SwayClient, Workspace, WorkspaceEvent};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, Orientation};
use ksway::{IpcCommand, IpcEvent};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tracing::error;

#[derive(Debug, Deserialize, Clone)]
pub struct WorkspacesModule {
    name_map: Option<HashMap<String, String>>,

    #[serde(default = "crate::config::default_false")]
    all_monitors: bool,
}

impl Workspace {
    fn as_button(&self, name_map: &HashMap<String, String>, tx: &mpsc::Sender<String>) -> Button {
        let button = Button::builder()
            .label(name_map.get(self.name.as_str()).unwrap_or(&self.name))
            .build();

        let style_context = button.style_context();
        style_context.add_class("item");

        if self.focused {
            style_context.add_class("focused");
        }

        {
            let tx = tx.clone();
            let name = self.name.clone();
            button.connect_clicked(move |_item| {
                tx.try_send(name.clone())
                    .expect("Failed to send workspace click event");
            });
        }

        button
    }
}

impl Module<gtk::Box> for WorkspacesModule {
    fn into_widget(self, info: &ModuleInfo) -> Result<gtk::Box> {
        let mut sway = SwayClient::connect()?;

        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let workspaces = {
            let raw = sway.ipc(IpcCommand::GetWorkspaces)?;
            let workspaces = serde_json::from_slice::<Vec<Workspace>>(&raw)?;

            if self.all_monitors {
                workspaces
            } else {
                workspaces
                    .into_iter()
                    .filter(|workspace| workspace.output == info.output_name)
                    .collect()
            }
        };

        let name_map = self.name_map.unwrap_or_default();

        let mut button_map: HashMap<String, Button> = HashMap::new();

        let (ui_tx, mut ui_rx) = mpsc::channel(32);

        for workspace in workspaces {
            let item = workspace.as_button(&name_map, &ui_tx);
            container.add(&item);
            button_map.insert(workspace.name, item);
        }

        let srx = sway.subscribe(vec![IpcEvent::Workspace])?;
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        spawn_blocking(move || loop {
            while let Ok((_, payload)) = srx.try_recv() {
                match serde_json::from_slice::<WorkspaceEvent>(&payload) {
                    Ok(payload) => tx.send(payload).expect("Failed to send workspace event"),
                    Err(err) => error!("{:?}", err),
                }
            }

            if let Err(err) = sway.poll() {
                error!("{:?}", err);
            }
        });

        {
            let menubar = container.clone();
            let output_name = info.output_name.to_string();
            rx.attach(None, move |event| {
                match event.change.as_str() {
                    "focus" => {
                        let old = event.old.and_then(|old| button_map.get(&old.name));
                        if let Some(old) = old {
                            old.style_context().remove_class("focused");
                        }

                        let new = event.current.and_then(|new| button_map.get(&new.name));
                        if let Some(new) = new {
                            new.style_context().add_class("focused");
                        }
                    }
                    "init" => {
                        if let Some(workspace) = event.current {
                            if self.all_monitors || workspace.output == output_name {
                                let item = workspace.as_button(&name_map, &ui_tx);

                                item.show();
                                menubar.add(&item);
                                button_map.insert(workspace.name, item);
                            }
                        }
                    }
                    "move" => {
                        if let Some(workspace) = event.current {
                            if !self.all_monitors {
                                if workspace.output == output_name {
                                    let item = workspace.as_button(&name_map, &ui_tx);

                                    item.show();
                                    menubar.add(&item);
                                    button_map.insert(workspace.name, item);
                                } else if let Some(item) = button_map.get(&workspace.name) {
                                    menubar.remove(item);
                                }
                            }
                        }
                    }
                    "empty" => {
                        if let Some(workspace) = event.current {
                            if let Some(item) = button_map.get(&workspace.name) {
                                menubar.remove(item);
                            }
                        }
                    }
                    _ => {}
                }

                Continue(true)
            });
        }

        spawn(async move {
            let mut sway = SwayClient::connect()?;
            while let Some(name) = ui_rx.recv().await {
                sway.run(format!("workspace {}", name))?;
            }

            Ok::<(), Report>(())
        });

        Ok(container)
    }
}
