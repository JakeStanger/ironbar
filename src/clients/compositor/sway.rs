use super::{Visibility, Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::await_sync;
use crate::channels::SyncSenderExt;
use crate::clients::sway::Client;
use color_eyre::Result;
use swayipc_async::{Node, WorkspaceChange, WorkspaceEvent};
use tokio::sync::broadcast::{channel, Receiver};

impl WorkspaceClient for Client {
    fn focus(&self, id: String) -> Result<()> {
        await_sync(async move {
            let mut client = self.connection().lock().await;
            client.run_command(format!("workspace {id}")).await
        })?;
        Ok(())
    }

    fn subscribe_workspace_change(&self) -> Receiver<WorkspaceUpdate> {
        let (tx, rx) = channel(16);

        let client = self.connection().clone();

        await_sync(async {
            let mut client = client.lock().await;
            let workspaces = client.get_workspaces().await.expect("to get workspaces");

            let event =
                WorkspaceUpdate::Init(workspaces.into_iter().map(Workspace::from).collect());

            tx.send_expect(event);

            drop(client);

            self.add_listener::<swayipc_async::WorkspaceEvent>(move |event| {
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
            name: node.name.unwrap_or_default(),
            monitor: node.output.unwrap_or_default(),
            visibility,
        }
    }
}

impl From<swayipc_async::Workspace> for Workspace {
    fn from(workspace: swayipc_async::Workspace) -> Self {
        let visibility = Visibility::from(&workspace);

        Self {
            id: workspace.id,
            name: workspace.name,
            monitor: workspace.output,
            visibility,
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
