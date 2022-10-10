use async_once::AsyncOnce;
use color_eyre::Report;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use std::sync::Arc;
use swayipc_async::{Connection, Event, EventType, WorkspaceEvent};
use tokio::spawn;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tracing::{info, trace};

pub struct SwayEventClient {
    workspace_tx: Sender<Box<WorkspaceEvent>>,
    _workspace_rx: Receiver<Box<WorkspaceEvent>>,
}

impl SwayEventClient {
    fn new() -> Self {
        let (workspace_tx, workspace_rx) = channel(16);

        let workspace_tx2 = workspace_tx.clone();

        spawn(async move {
            let workspace_tx = workspace_tx2;

            let client = Connection::new().await?;
            info!("Sway IPC subscription client connected");

            let event_types = [EventType::Workspace];

            let mut events = client.subscribe(event_types).await?;

            while let Some(event) = events.next().await {
                trace!("event: {:?}", event);
                if let Event::Workspace(ev) = event? {
                    workspace_tx.send(ev)?;
                };
            }

            Ok::<(), Report>(())
        });

        Self {
            workspace_tx,
            _workspace_rx: workspace_rx,
        }
    }

    /// Gets an event receiver for workspace events
    pub fn subscribe_workspace(&self) -> Receiver<Box<WorkspaceEvent>> {
        self.workspace_tx.subscribe()
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
pub async fn get_client() -> Arc<Mutex<Connection>> {
    let client = CLIENT.get().await;
    Arc::clone(client)
}

/// Gets the sway IPC event subscription client
pub fn get_sub_client() -> &'static SwayEventClient {
    &SUB_CLIENT
}
