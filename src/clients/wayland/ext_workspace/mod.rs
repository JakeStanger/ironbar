use super::{Client, Request, Response};
use crate::channels::SyncSenderExt;
use tokio::sync::broadcast;

pub use wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::{
    State, WorkspaceCapabilities,
};

pub mod group_handle;
pub mod handle;
pub mod manager;

#[derive(Debug, Default, Clone)]
pub struct WorkspaceGroup {
    pub output: Option<String>,
    pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub coordinates: Vec<u32>,
    pub state: State,
    pub capabilities: WorkspaceCapabilities,
}

#[cfg(feature = "workspaces")]
impl Client {
    pub fn workspace_info_all(&self) -> Vec<crate::clients::compositor::Workspace> {
        match self.send_request(Request::WorkspaceInfoAll) {
            Response::WorkspaceInfoAll(workspaces) => workspaces,
            _ => vec![],
        }
    }

    pub fn subscribe_workspaces(
        &self,
    ) -> broadcast::Receiver<crate::clients::compositor::WorkspaceUpdate> {
        self.workspace_channel.0.subscribe()
    }
}

#[cfg(feature = "workspaces")]
impl crate::clients::compositor::WorkspaceClient for Client {
    fn focus(&self, id: i64) {
        self.activate_exclusive(id);
    }

    fn activate(&self, id: i64) {
        self.send_request(Request::WorkspaceActivate(id));
    }

    fn deactivate(&self, id: i64) {
        self.send_request(Request::WorkspaceDeactivate(id));
    }

    fn toggle(&self, id: i64) {
        self.send_request(Request::WorkspaceToggle(id));
    }

    fn activate_exclusive(&self, id: i64) {
        self.send_request(Request::WorkspaceActivateExclusive(id));
    }

    fn subscribe(&self) -> broadcast::Receiver<crate::clients::compositor::WorkspaceUpdate> {
        let rx = self.subscribe_workspaces();
        let workspaces = self.workspace_info_all();
        self.workspace_channel
            .0
            .send_expect(crate::clients::compositor::WorkspaceUpdate::Init(
                workspaces,
            ));
        rx
    }
}
