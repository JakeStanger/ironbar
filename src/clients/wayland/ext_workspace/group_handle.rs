use super::manager::WorkspaceManagerState;
use std::sync::{Arc, Mutex};
use tracing::error;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::{ExtWorkspaceGroupHandleV1, Event};

#[derive(Debug, Clone)]
pub struct WorkspaceGroupHandle {
    pub handle: ExtWorkspaceGroupHandleV1,
}

impl PartialEq for WorkspaceGroupHandle {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl WorkspaceGroupHandle {}

#[derive(Debug, Default)]
pub struct WorkspaceGroupHandleData {
    pub inner: Arc<Mutex<WorkspaceGroupHandleDataInner>>,
}

impl WorkspaceGroupHandleData {}

#[derive(Debug, Default)]
pub struct WorkspaceGroupHandleDataInner {}

pub trait WorkspaceGroupHandleDataExt {}

impl WorkspaceGroupHandleDataExt for WorkspaceGroupHandleData {}

pub trait WorkspaceGroupHandleHandler: Sized {}

impl<D, U> Dispatch<ExtWorkspaceGroupHandleV1, U, D> for WorkspaceManagerState
where
    D: Dispatch<ExtWorkspaceGroupHandleV1, U> + 'static,
    U: WorkspaceGroupHandleDataExt,
{
    fn event(
        _state: &mut D,
        proxy: &ExtWorkspaceGroupHandleV1,
        event: Event,
        _data: &U,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        println!("GROUP HANDLE EVENT: {event:?}");

        match event {
            Event::Capabilities { capabilities } => {}
            Event::OutputEnter { output } => {}
            Event::OutputLeave { output } => {}
            Event::WorkspaceEnter { workspace } => {}
            Event::WorkspaceLeave { workspace } => {}
            Event::Removed => proxy.destroy(),
            _ => {
                error!("received unimplemented event {event:?}");
            }
        }
    }
}
