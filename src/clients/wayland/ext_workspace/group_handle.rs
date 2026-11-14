use super::manager::WorkspaceManagerState;
use crate::lock;
use std::sync::{Arc, Mutex};
use tracing::{error, trace};
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::{
    Event, ExtWorkspaceGroupHandleV1,
};

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

impl WorkspaceGroupHandleData {
    pub fn outputs(&self) -> Vec<WlOutput> {
        lock!(self.inner).outputs.clone()
    }
}

#[derive(Debug, Default)]
pub struct WorkspaceGroupHandleDataInner {
    outputs: Vec<WlOutput>,
}

pub trait WorkspaceGroupHandleDataExt {
    fn workspace_group_handle_data(&self) -> &WorkspaceGroupHandleData;
}

impl WorkspaceGroupHandleDataExt for WorkspaceGroupHandleData {
    fn workspace_group_handle_data(&self) -> &WorkspaceGroupHandleData {
        self
    }
}

pub trait WorkspaceGroupHandleHandler: Sized {
    fn workspace_group_output_enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        group: WorkspaceGroupHandle,
        output: WlOutput,
    );
    fn workspace_group_output_leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        group: WorkspaceGroupHandle,
        output: WlOutput,
    );
    fn workspace_group_workspace_enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        group: WorkspaceGroupHandle,
        workspace: wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    );
    fn workspace_group_workspace_leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        group: WorkspaceGroupHandle,
        workspace: wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    );
    fn workspace_group_removed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        group: WorkspaceGroupHandle,
    );
}

impl<D, U> Dispatch<ExtWorkspaceGroupHandleV1, U, D> for WorkspaceManagerState
where
    D: Dispatch<ExtWorkspaceGroupHandleV1, U> + WorkspaceGroupHandleHandler + 'static,
    U: WorkspaceGroupHandleDataExt,
{
    fn event(
        state: &mut D,
        proxy: &ExtWorkspaceGroupHandleV1,
        event: Event,
        data: &U,
        conn: &Connection,
        qhandle: &QueueHandle<D>,
    ) {
        let data = data.workspace_group_handle_data();

        match event {
            Event::Capabilities { capabilities } => {
                trace!("workspace group capabilities: {capabilities:?}");
            }
            Event::OutputEnter { output } => {
                let mut inner = lock!(data.inner);
                if !inner.outputs.iter().any(|existing| existing == &output) {
                    inner.outputs.push(output.clone());
                }
                drop(inner);
                state.workspace_group_output_enter(
                    conn,
                    qhandle,
                    WorkspaceGroupHandle {
                        handle: proxy.clone(),
                    },
                    output,
                );
            }
            Event::OutputLeave { output } => {
                lock!(data.inner)
                    .outputs
                    .retain(|existing| existing != &output);
                state.workspace_group_output_leave(
                    conn,
                    qhandle,
                    WorkspaceGroupHandle {
                        handle: proxy.clone(),
                    },
                    output,
                );
            }
            Event::WorkspaceEnter { workspace } => {
                state.workspace_group_workspace_enter(
                    conn,
                    qhandle,
                    WorkspaceGroupHandle {
                        handle: proxy.clone(),
                    },
                    workspace,
                );
            }
            Event::WorkspaceLeave { workspace } => {
                state.workspace_group_workspace_leave(
                    conn,
                    qhandle,
                    WorkspaceGroupHandle {
                        handle: proxy.clone(),
                    },
                    workspace,
                );
            }
            Event::Removed => {
                state.workspace_group_removed(
                    conn,
                    qhandle,
                    WorkspaceGroupHandle {
                        handle: proxy.clone(),
                    },
                );
                proxy.destroy();
            }
            _ => {
                error!("received unimplemented event {event:?}");
            }
        }
    }
}
