use super::group_handle::{WorkspaceGroupHandleData, WorkspaceGroupHandleHandler};
use super::handle::{WorkspaceHandleData, WorkspaceHandleHandler};
use smithay_client_toolkit::error::GlobalError;
use smithay_client_toolkit::globals::{GlobalData, ProvidesBoundGlobal};
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
}

impl WorkspaceManagerState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<ExtWorkspaceManagerV1, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=1, GlobalData)?;
        debug!("Bound to ExtWorkspaceManagerV1 global");

        Ok(Self { manager })
    }

    pub fn commit(&self) {
        self.manager.commit();
    }
}

pub trait WorkspaceManagerHandler: Sized {
    fn workspace_group_created(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        group: ExtWorkspaceGroupHandleV1,
    );
    fn workspace_created(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        workspace: ExtWorkspaceHandleV1,
    );
    fn workspace_done(&mut self, conn: &Connection, qh: &QueueHandle<Self>);
    fn workspace_finished(&mut self, conn: &Connection, qh: &QueueHandle<Self>);
}

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
        conn: &Connection,
        qhandle: &QueueHandle<D>,
    ) {
        match event {
            Event::WorkspaceGroup { workspace_group } => {
                state.workspace_group_created(conn, qhandle, workspace_group);
            }
            Event::Workspace { workspace } => {
                state.workspace_created(conn, qhandle, workspace);
            }
            Event::Done => state.workspace_done(conn, qhandle),
            Event::Finished => state.workspace_finished(conn, qhandle),
            _ => {
                error!("received unimplemented event {event:?}");
            }
        }
    }
}
