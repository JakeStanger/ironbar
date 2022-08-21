use color_eyre::{Report, Result};
use ksway::{Error, IpcCommand, IpcEvent};
use serde::Deserialize;

pub mod node;

#[derive(Deserialize, Debug)]
pub struct WorkspaceEvent {
    pub change: String,
    pub old: Option<Workspace>,
    pub current: Option<Workspace>,
}

#[derive(Deserialize, Debug)]
pub struct Workspace {
    pub name: String,
    pub focused: bool,
    // pub num: i32,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct WindowEvent {
    pub change: String,
    pub container: SwayNode,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct WindowProperties {
    pub class: Option<String>,
}

#[derive(Deserialize)]
pub struct SwayOutput {
    pub name: String,
}

pub struct SwayClient {
    client: ksway::Client,
}

impl SwayClient {
    pub(crate) fn run(&mut self, cmd: String) -> Result<Vec<u8>> {
        match self.client.run(cmd) {
            Ok(res) => Ok(res),
            Err(err) => Err(get_client_error(err)),
        }
    }
}

impl SwayClient {
    pub fn connect() -> Result<Self> {
        let client = match ksway::Client::connect() {
            Ok(client) => Ok(client),
            Err(err) => Err(get_client_error(err)),
        }?;

        Ok(Self { client })
    }

    pub fn ipc(&mut self, command: IpcCommand) -> Result<Vec<u8>> {
        match self.client.ipc(command) {
            Ok(res) => Ok(res),
            Err(err) => Err(get_client_error(err)),
        }
    }

    pub fn subscribe(
        &mut self,
        event_types: Vec<IpcEvent>,
    ) -> Result<crossbeam_channel::Receiver<(IpcEvent, Vec<u8>)>> {
        match self.client.subscribe(event_types) {
            Ok(res) => Ok(res),
            Err(err) => Err(get_client_error(err)),
        }
    }

    pub fn poll(&mut self) -> Result<()> {
        match self.client.poll() {
            Ok(()) => Ok(()),
            Err(err) => Err(get_client_error(err)),
        }
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
