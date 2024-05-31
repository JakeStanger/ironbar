use super::{Visibility, Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::{await_sync, send, spawn};
use color_eyre::{Report, Result};
use futures_lite::StreamExt;
use std::sync::Arc;
use swayipc_async::{Connection, Event, EventType, Node, WorkspaceChange, WorkspaceEvent};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tracing::{info, trace};

#[derive(Debug)]
pub struct Client {
    client: Arc<Mutex<Connection>>,
    workspace_tx: Sender<WorkspaceUpdate>,
    _workspace_rx: Receiver<WorkspaceUpdate>,
}

impl Client {
    pub(crate) async fn new() -> Result<Self> {
        // Avoid using `arc_mut!` here because we need tokio Mutex.
        let client = Arc::new(Mutex::new(Connection::new().await?));
        info!("Sway IPC subscription client connected");

        let (workspace_tx, workspace_rx) = channel(16);

        {
            // create 2nd client as subscription takes ownership
            let client = Connection::new().await?;
            let workspace_tx = workspace_tx.clone();

            spawn(async move {
                let event_types = [EventType::Workspace];
                let mut events = client.subscribe(event_types).await?;

                while let Some(event) = events.next().await {
                    trace!("event: {:?}", event);
                    if let Event::Workspace(event) = event? {
                        let event = WorkspaceUpdate::from(*event);
                        if !matches!(event, WorkspaceUpdate::Unknown) {
                            workspace_tx.send(event)?;
                        }
                    };
                }

                Ok::<(), Report>(())
            });
        }

        Ok(Self {
            client,
            workspace_tx,
            _workspace_rx: workspace_rx,
        })
    }
}

impl WorkspaceClient for Client {
    fn focus(&self, id: String) -> Result<()> {
        await_sync(async move {
            let mut client = self.client.lock().await;
            client.run_command(format!("workspace {id}")).await
        })?;
        Ok(())
    }

    fn subscribe_workspace_change(&self) -> Receiver<WorkspaceUpdate> {
        let rx = self.workspace_tx.subscribe();

        {
            let tx = self.workspace_tx.clone();
            let client = self.client.clone();

            await_sync(async {
                let mut client = client.lock().await;
                let workspaces = client.get_workspaces().await.expect("to get workspaces");

                let event =
                    WorkspaceUpdate::Init(workspaces.into_iter().map(Workspace::from).collect());

                send!(tx, event);
            });
        }

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
            _ => Self::Unknown,
        }
    }
}
