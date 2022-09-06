use crate::broadcaster::Broadcaster;
use color_eyre::{Report, Result};
use crossbeam_channel::Receiver;
use ksway::{Error, IpcCommand, IpcEvent};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::spawn;
use tracing::{debug, info, trace};

pub mod node;

#[derive(Deserialize, Debug, Clone)]
pub struct WorkspaceEvent {
    pub change: String,
    pub old: Option<Workspace>,
    pub current: Option<Workspace>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Workspace {
    pub name: String,
    pub focused: bool,
    // pub num: i32,
    pub output: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowEvent {
    pub change: String,
    pub container: SwayNode,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SwayNode {
    #[serde(rename = "type")]
    pub node_type: String,
    pub id: i32,
    pub name: Option<String>,
    pub app_id: Option<String>,
    pub focused: bool,
    pub urgent: bool,
    pub nodes: Vec<SwayNode>,
    pub floating_nodes: Vec<SwayNode>,
    pub shell: Option<String>,
    pub window_properties: Option<WindowProperties>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowProperties {
    pub class: Option<String>,
}

#[derive(Deserialize)]
pub struct SwayOutput {
    pub name: String,
}

type EventBroadcaster<T> = Arc<Mutex<Broadcaster<T>>>;

pub struct SwayClient {
    client: ksway::Client,

    workspace_bc: EventBroadcaster<WorkspaceEvent>,
    window_bc: EventBroadcaster<WindowEvent>,
}

impl SwayClient {
    fn connect() -> Result<Self> {
        let client = match ksway::Client::connect() {
            Ok(client) => Ok(client),
            Err(err) => Err(get_client_error(err)),
        }?;
        info!("Sway IPC client connected");

        let workspace_bc = Arc::new(Mutex::new(Broadcaster::new()));
        let window_bc = Arc::new(Mutex::new(Broadcaster::new()));

        let workspace_bc2 = workspace_bc.clone();
        let window_bc2 = window_bc.clone();
        spawn(async move {
            let mut sub_client = match ksway::Client::connect() {
                Ok(client) => Ok(client),
                Err(err) => Err(get_client_error(err)),
            }
            .expect("Failed to connect to Sway IPC server");
            info!("Sway IPC subscription client connected");

            let event_types = vec![IpcEvent::Window, IpcEvent::Workspace];
            let rx = match sub_client.subscribe(event_types) {
                Ok(res) => Ok(res),
                Err(err) => Err(get_client_error(err)),
            }
            .expect("Failed to subscribe to Sway IPC server");

            loop {
                while let Ok((ev_type, payload)) = rx.try_recv() {
                    debug!("Received sway event {:?}", ev_type);
                    match ev_type {
                        IpcEvent::Workspace => {
                            let json = serde_json::from_slice::<WorkspaceEvent>(&payload).expect(
                                "Received invalid workspace event payload from Sway IPC server",
                            );
                            workspace_bc
                                .lock()
                                .expect("Failed to get lock on workspace event bus")
                                .send(json)
                                .expect("Failed to broadcast workspace event");
                        }
                        IpcEvent::Window => {
                            let json = serde_json::from_slice::<WindowEvent>(&payload).expect(
                                "Received invalid window event payload from Sway IPC server",
                            );
                            window_bc
                                .lock()
                                .expect("Failed to get lock on window event bus")
                                .send(json)
                                .expect("Failed to broadcast window event");
                        }
                        _ => {}
                    }
                }
                match sub_client.poll() {
                    Ok(()) => Ok(()),
                    Err(err) => Err(get_client_error(err)),
                }
                .expect("Failed to poll Sway IPC client");
            }
        });

        Ok(Self {
            client,
            workspace_bc: workspace_bc2,
            window_bc: window_bc2,
        })
    }

    pub fn ipc(&mut self, command: IpcCommand) -> Result<Vec<u8>> {
        debug!("Sending command: {:?}", command);
        match self.client.ipc(command) {
            Ok(res) => Ok(res),
            Err(err) => Err(get_client_error(err)),
        }
    }

    pub(crate) fn run(&mut self, cmd: String) -> Result<Vec<u8>> {
        debug!("Sending command: {}", cmd);
        match self.client.run(cmd) {
            Ok(res) => Ok(res),
            Err(err) => Err(get_client_error(err)),
        }
    }

    pub fn subscribe_workspace(&mut self) -> Receiver<WorkspaceEvent> {
        trace!("Adding new workspace subscriber");
        self.workspace_bc
            .lock()
            .expect("Failed to get lock on workspace event bus")
            .subscribe()
    }

    pub fn subscribe_window(&mut self) -> Receiver<WindowEvent> {
        trace!("Adding new window subscriber");
        self.window_bc
            .lock()
            .expect("Failed to get lock on window event bus")
            .subscribe()
    }
}

/// Gets an error report from a `ksway` error enum variant
pub fn get_client_error(error: Error) -> Report {
    match error {
        Error::SockPathNotFound => Report::msg("Sway socket path not found"),
        Error::SubscriptionError => Report::msg("Sway IPC subscription error"),
        Error::AlreadySubscribed => Report::msg("Already subscribed to Sway IPC server"),
        Error::Io(err) => Report::new(err),
    }
}

lazy_static! {
    static ref CLIENT: Arc<Mutex<SwayClient>> = {
        let client = SwayClient::connect();
        match client {
            Ok(client) => Arc::new(Mutex::new(client)),
            Err(err) => panic!("{:?}", err),
        }
    };
}

pub fn get_client() -> Arc<Mutex<SwayClient>> {
    Arc::clone(&CLIENT)
}
