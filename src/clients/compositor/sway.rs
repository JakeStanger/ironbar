use super::{Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::{await_sync, send};
use async_once::AsyncOnce;
use color_eyre::Report;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use std::sync::Arc;
use swayipc_async::{Connection, Event, EventType, Node, WorkspaceChange, WorkspaceEvent};
use tokio::spawn;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tracing::{info, trace};

pub struct SwayEventClient {
    workspace_tx: Sender<WorkspaceUpdate>,
    _workspace_rx: Receiver<WorkspaceUpdate>,
}

impl SwayEventClient {
    fn new() -> Self {
        let (workspace_tx, workspace_rx) = channel(16);

        {
            let workspace_tx = workspace_tx.clone();
            spawn(async move {
                let client = Connection::new().await?;
                info!("Sway IPC subscription client connected");

                let event_types = [EventType::Workspace];

                let mut events = client.subscribe(event_types).await?;

                while let Some(event) = events.next().await {
                    trace!("event: {:?}", event);
                    if let Event::Workspace(ev) = event? {
                        workspace_tx.send(WorkspaceUpdate::from(*ev))?;
                    };
                }

                Ok::<(), Report>(())
            });
        }

        Self {
            workspace_tx,
            _workspace_rx: workspace_rx,
        }
    }
}

impl WorkspaceClient for SwayEventClient {
    fn focus(&self, id: String) -> color_eyre::Result<()> {
        await_sync(async move {
            let client = get_client().await;
            let mut client = client.lock().await;
            client.run_command(format!("workspace {id}")).await
        })?;
        Ok(())
    }

    fn subscribe_workspace_change(&self) -> Receiver<WorkspaceUpdate> {
        let rx = self.workspace_tx.subscribe();

        {
            let tx = self.workspace_tx.clone();
            await_sync(async {
                let client = get_client().await;
                let mut client = client.lock().await;

                let workspaces = client
                    .get_workspaces()
                    .await
                    .expect("Failed to get workspaces");
                let event =
                    WorkspaceUpdate::Init(workspaces.into_iter().map(Workspace::from).collect());

                send!(tx, event);
            });
        }

        rx
    }
}

lazy_static! {
    static ref CLIENT: AsyncOnce<Arc<Mutex<Connection>>> = AsyncOnce::new(async {
        let client = Connection::new()
            .await
            .expect("Failed to connect to Sway socket");
        Arc::new(Mutex::new(client))
    });
    static ref SUB_CLIENT: SwayEventClient = SwayEventClient::new();
}

/// Gets the sway IPC client
async fn get_client() -> Arc<Mutex<Connection>> {
    let client = CLIENT.get().await;
    Arc::clone(client)
}

/// Gets the sway IPC event subscription client
pub fn get_sub_client() -> &'static SwayEventClient {
    &SUB_CLIENT
}

impl From<Node> for Workspace {
    fn from(node: Node) -> Self {
        Self {
            id: node.id.to_string(),
            name: node.name.unwrap_or_default(),
            monitor: node.output.unwrap_or_default(),
            focused: node.focused,
        }
    }
}

impl From<swayipc_async::Workspace> for Workspace {
    fn from(workspace: swayipc_async::Workspace) -> Self {
        Self {
            id: workspace.id.to_string(),
            name: workspace.name,
            monitor: workspace.output,
            focused: workspace.focused,
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
                Self::Remove(event.current.expect("Missing current workspace").into())
            }
            WorkspaceChange::Focus => Self::Focus {
                old: event.old.expect("Missing old workspace").into(),
                new: event.current.expect("Missing current workspace").into(),
            },
            WorkspaceChange::Move => {
                Self::Move(event.current.expect("Missing current workspace").into())
            }
            _ => Self::Update(event.current.expect("Missing current workspace").into()),
        }
    }
}
