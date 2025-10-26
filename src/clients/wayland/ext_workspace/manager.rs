use super::group_handle::{WorkspaceGroupHandleData, WorkspaceGroupHandleHandler};
use super::handle::{WorkspaceHandleData, WorkspaceHandleHandler};
use super::{Workspace, WorkspaceGroup};
use crate::arc_mut;
use smithay_client_toolkit::error::GlobalError;
use smithay_client_toolkit::globals::{GlobalData, ProvidesBoundGlobal};
use std::sync::{Arc, Mutex};
use tracing::{debug, error};
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    ext_workspace_manager_v1::{Event, ExtWorkspaceManagerV1},
};

#[derive(Debug)]
pub struct WorkspaceManagerState {
    manager: ExtWorkspaceManagerV1,
    pending: Vec<WorkspaceGroup>,
}

#[derive(Debug, Default)]
struct PendingState {
    groups: Vec<WorkspaceGroup>,
    workspaces: Vec<Workspace>,
}

impl WorkspaceManagerState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<ExtWorkspaceManagerV1, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=1, GlobalData)?;
        debug!("Bound to ExtWorkspaceManagerV1 global");

        Ok(Self {
            manager,
            pending: vec![],
        })
    }
}

pub trait WorkspaceManagerHandler: Sized {}

impl ProvidesBoundGlobal<ExtWorkspaceManagerV1, 1> for WorkspaceManagerState {
    fn bound_global(&self) -> Result<ExtWorkspaceManagerV1, GlobalError> {
        Ok(self.manager.clone())
    }
}

impl<D> Dispatch<ExtWorkspaceManagerV1, GlobalData, D> for WorkspaceManagerState
where
    D: Dispatch<ExtWorkspaceManagerV1, GlobalData>
        + Dispatch<ExtWorkspaceGroupHandleV1, WorkspaceGroupHandleData>
        + Dispatch<ExtWorkspaceHandleV1, WorkspaceHandleData>
        + WorkspaceManagerHandler
        + WorkspaceGroupHandleHandler
        + WorkspaceHandleHandler
        + 'static,
{
    event_created_child!(D, ExtWorkspaceManagerV1, [
        0 => (ExtWorkspaceGroupHandleV1, WorkspaceGroupHandleData::default()),
        1 => (ExtWorkspaceHandleV1, WorkspaceHandleData::default())
    ]);

    fn event(
        state: &mut D,
        _proxy: &ExtWorkspaceManagerV1,
        event: Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        println!("EVENT: {event:?}");

        match event {
            Event::WorkspaceGroup { workspace_group } => {}
            Event::Workspace { workspace } => {}
            Event::Done => {}
            Event::Finished => {}
            _ => {
                error!("received unimplemented event {event:?}");
            }
        }
    }
}
