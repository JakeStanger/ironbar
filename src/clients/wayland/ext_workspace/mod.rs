use super::{Client, Environment};
use group_handle::WorkspaceGroupHandleHandler;
use handle::WorkspaceHandleHandler;
use manager::WorkspaceManagerHandler;

pub use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::GroupCapabilities,
    ext_workspace_handle_v1::{State, WorkspaceCapabilities},
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

impl Client {}

impl WorkspaceManagerHandler for Environment {}

impl WorkspaceGroupHandleHandler for Environment {}

impl WorkspaceHandleHandler for Environment {}
