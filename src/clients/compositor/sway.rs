use super::{Visibility, Workspace};
use crate::channels::SyncSenderExt;
use crate::clients::sway::Client;
use crate::{await_sync, error, spawn};
use color_eyre::Report;
use swayipc_async::{InputChange, InputEvent, Node, WorkspaceChange, WorkspaceEvent};
use tokio::sync::broadcast::{Receiver, channel};

#[cfg(feature = "workspaces")]
use super::WorkspaceUpdate;

#[cfg(feature = "workspaces+sway")]
impl super::WorkspaceClient for Client {
    fn focus(&self, id: i64) {
        let client = self.connection().clone();
        spawn(async move {
            let mut client = client.lock().await;

            let name = client
                .get_workspaces()
                .await?
                .into_iter()
                .find(|w| w.id == id)
                .map(|w| w.name);

            let Some(name) = name else {
                return Err(Report::msg(format!("couldn't find workspace with id {id}")));
            };

            if let Err(e) = client.run_command(format!("workspace {name}")).await {
                return Err(Report::msg(format!(
                    "Couldn't focus workspace '{id}': {e:#}"
                )));
            }

            Ok(())
        });
    }

    fn subscribe(&self) -> Receiver<WorkspaceUpdate> {
        let (tx, rx) = channel(16);

        let client = self.connection().clone();

        // TODO: this needs refactoring
        await_sync(async {
            let mut client = client.lock().await;
            let workspaces = client.get_workspaces().await.expect("to get workspaces");

            let event =
                WorkspaceUpdate::Init(workspaces.into_iter().map(Workspace::from).collect());

            tx.send_expect(event);

            drop(client);

            self.add_listener::<WorkspaceEvent>(move |event| {
                let update = WorkspaceUpdate::from(event.clone());
                tx.send_expect(update);
            })
            .await
            .expect("to add listener");
        });

        rx
    }
}

impl From<Node> for Workspace {
    fn from(node: Node) -> Self {
        let visibility = Visibility::from(&node);

        Self {
            id: node.id,
            index: node.num.unwrap_or(0) as i64,
            name: node.name.unwrap_or_default(),
            monitor: node.output.unwrap_or_default(),
            visibility,
            windows: 0,
        }
    }
}

impl From<swayipc_async::Workspace> for Workspace {
    fn from(workspace: swayipc_async::Workspace) -> Self {
        let visibility = Visibility::from(&workspace);

        Self {
            id: workspace.id,
            index: workspace.num as i64,
            name: workspace.name,
            monitor: workspace.output,
            visibility,
            windows: 0,
        }
    }
}

impl From<&Node> for Visibility {
    fn from(node: &Node) -> Self {
        if node.focused {
            Self::focused()
        } else if node.visible.unwrap_or(false) {
            Self::visible()
        } else {
            Self::Hidden
        }
    }
}

impl From<&swayipc_async::Workspace> for Visibility {
    fn from(workspace: &swayipc_async::Workspace) -> Self {
        if workspace.focused {
            Self::focused()
        } else if workspace.visible {
            Self::visible()
        } else {
            Self::Hidden
        }
    }
}

#[cfg(feature = "workspaces")]
impl From<WorkspaceEvent> for WorkspaceUpdate {
    fn from(event: WorkspaceEvent) -> Self {
        match event.change {
            WorkspaceChange::Init => {
                Self::Add(event.current.expect("Missing current workspace").into())
            }
            WorkspaceChange::Empty => {
                Self::Remove(event.current.expect("Missing current workspace").id)
            }
            WorkspaceChange::Focus => Self::Focus {
                old: event.old.map(Workspace::from),
                new: Workspace::from(event.current.expect("Missing current workspace")),
            },
            WorkspaceChange::Move => {
                Self::Move(event.current.expect("Missing current workspace").into())
            }
            WorkspaceChange::Rename => {
                if let Some(node) = event.current {
                    Self::Rename {
                        id: node.id,
                        name: node.name.unwrap_or_default(),
                    }
                } else {
                    Self::Unknown
                }
            }
            WorkspaceChange::Urgent => {
                if let Some(node) = event.current {
                    Self::Urgent {
                        id: node.id,
                        urgent: node.urgent,
                    }
                } else {
                    Self::Unknown
                }
            }
            _ => Self::Unknown,
        }
    }
}

#[cfg(feature = "keyboard+sway")]
use super::{KeyboardLayoutClient, KeyboardLayoutUpdate};

#[cfg(feature = "keyboard+sway")]
impl KeyboardLayoutClient for Client {
    fn set_next_active(&self) {
        let client = self.connection().clone();
        spawn(async move {
            let mut client = client.lock().await;

            let inputs = client.get_inputs().await.expect("to get inputs");

            if let Some(keyboard) = inputs
                .into_iter()
                .find(|i| i.xkb_active_layout_name.is_some())
            {
                if let Err(e) = client
                    .run_command(format!(
                        "input {} xkb_switch_layout next",
                        keyboard.identifier
                    ))
                    .await
                {
                    error!("Failed to switch keyboard layout due to Sway error: {e}");
                }
            } else {
                error!("Failed to get keyboard identifier from Sway");
            }
        });
    }

    fn subscribe(&self) -> Receiver<KeyboardLayoutUpdate> {
        let (tx, rx) = channel(16);

        let client = self.connection().clone();

        await_sync(async {
            let mut client = client.lock().await;
            let inputs = client.get_inputs().await.expect("to get inputs");

            if let Some(layout) = inputs.into_iter().find_map(|i| i.xkb_active_layout_name) {
                tx.send_expect(KeyboardLayoutUpdate(layout));
            } else {
                error!("Failed to get keyboard layout from Sway!");
            }

            drop(client);

            self.add_listener::<InputEvent>(move |event| {
                if let Ok(layout) = KeyboardLayoutUpdate::try_from(event.clone()) {
                    tx.send_expect(layout);
                }
            })
            .await
            .expect("to add listener");
        });

        rx
    }
}

#[cfg(feature = "keyboard+sway")]
impl TryFrom<InputEvent> for KeyboardLayoutUpdate {
    type Error = ();

    fn try_from(value: InputEvent) -> Result<Self, Self::Error> {
        match value.change {
            InputChange::XkbLayout | InputChange::XkbKeymap => {
                if let Some(layout) = value.input.xkb_active_layout_name {
                    Ok(KeyboardLayoutUpdate(layout))
                } else {
                    Err(())
                }
            }
            _ => Err(()),
        }
    }
}

#[cfg(feature = "bindmode+sway")]
use super::{BindModeClient, BindModeUpdate};

#[cfg(feature = "bindmode+sway")]
impl BindModeClient for Client {
    fn subscribe(&self) -> super::Result<Receiver<BindModeUpdate>> {
        let (tx, rx) = channel(16);

        await_sync(async {
            self.add_listener::<swayipc_async::ModeEvent>(move |mode| {
                tracing::trace!("mode: {:?}", mode);

                // when no binding is active the bindmode is named "default", but we must display
                // nothing in this case.
                let name = if mode.change == "default" {
                    String::new()
                } else {
                    mode.change.clone()
                };

                tx.send_expect(BindModeUpdate {
                    name,
                    pango_markup: mode.pango_markup,
                });
            })
            .await
        })
        .map_err(|err| super::Error::Other(err.into()))?;

        Ok(rx)
    }
}
