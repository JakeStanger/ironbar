use crate::clients::wayland::ext_workspace::manager::WorkspaceManagerState;
use crate::lock;
use bluer::adv::Capabilities;
use std::sync::{Arc, Mutex};
use tracing::{error, warn};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    ext_workspace_handle_v1::{Event, ExtWorkspaceHandleV1},
};

pub use wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::{
    State, WorkspaceCapabilities,
};

#[derive(Debug, Clone)]
pub struct WorkspaceHandle {
    pub handle: ExtWorkspaceHandleV1,
}

impl PartialEq for WorkspaceHandle {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl WorkspaceHandle {
    fn activate(&self) {
        self.handle.activate();
    }

    fn deactivate(&self) {
        self.handle.deactivate();
    }

    fn assign(&self, group: &ExtWorkspaceGroupHandleV1) {
        self.handle.assign(group);
    }
}

#[derive(Debug, Default)]
pub struct WorkspaceHandleData {
    pub inner: Arc<Mutex<WorkspaceHandleDataInner>>,
}

impl WorkspaceHandleData {}

#[derive(Debug)]
pub struct WorkspaceHandleDataInner {
    id: String,
    name: String,
    coordinates: Vec<u32>,
    state: State,
    capabilities: WorkspaceCapabilities,
}

impl Default for WorkspaceHandleDataInner {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            coordinates: vec![],
            state: State::empty(),
            capabilities: WorkspaceCapabilities::empty(),
        }
    }
}

pub trait WorkspaceHandleDataExt {
    fn workspace_handle_data(&self) -> &WorkspaceHandleData;
}

impl WorkspaceHandleDataExt for WorkspaceHandleData {
    fn workspace_handle_data(&self) -> &WorkspaceHandleData {
        self
    }
}

pub trait WorkspaceHandleHandler: Sized {

}

impl<D, U> Dispatch<ExtWorkspaceHandleV1, U, D> for WorkspaceManagerState
where
    D: Dispatch<ExtWorkspaceHandleV1, U> + WorkspaceHandleHandler,
    U: WorkspaceHandleDataExt,
{
    fn event(
        state: &mut D,
        proxy: &ExtWorkspaceHandleV1,
        event: Event,
        data: &U,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        println!("HANDLE EVENT: {event:?}");

        let data = data.workspace_handle_data();

        match event {
            Event::Id { id } => lock!(data.inner).id = id,
            Event::Name { name } => lock!(data.inner).name = name,
            Event::Coordinates { coordinates } => {
                // received as a `Vec<u8>` where every 4 bytes make up a `u32`
                assert_eq!(coordinates.len() % 4, 0);
                let coordinates = (0..coordinates.len() / 4)
                    .map(|i| {
                        let slice: [u8; 4] = coordinates[i * 4..i * 4 + 4]
                            .try_into()
                            .expect("Received invalid state length");
                        u32::from_le_bytes(slice)
                    })
                    .collect::<Vec<_>>();
                lock!(data.inner).coordinates = coordinates;
            }
            Event::State { state } => match state {
                WEnum::Value(state) => lock!(data.inner).state = state,
                WEnum::Unknown(value) => warn!("received unknown state: {value}",),
            },
            Event::Capabilities { capabilities } => match capabilities {
                WEnum::Value(capabilities) => lock!(data.inner).capabilities = capabilities,
                WEnum::Unknown(value) => warn!("received unknown capabilities: {value}"),
            },
            Event::Removed => proxy.destroy(),
            _ => {
                error!("received unimplemented event {event:?}");
            }
        }
    }
}
