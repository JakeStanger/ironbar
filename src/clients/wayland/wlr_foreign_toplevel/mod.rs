pub mod handle;
pub mod manager;

use self::handle::ToplevelHandleHandler;
use self::manager::{ToplevelManagerHandler, ToplevelManagerState};
use crate::clients::wayland::Environment;
use tracing::{debug, error, trace};
use wayland_client::{Connection, QueueHandle};

use crate::send;
pub use handle::{ToplevelHandle, ToplevelInfo};

#[derive(Debug, Clone)]
pub enum ToplevelEvent {
    New(ToplevelHandle),
    Update(ToplevelHandle),
    Remove(ToplevelHandle),
}

impl ToplevelManagerHandler for Environment {
    fn toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _manager: ToplevelManagerState,
    ) {
        debug!("Manager received new handle");
    }
}

impl ToplevelHandleHandler for Environment {
    fn new_handle(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, handle: ToplevelHandle) {
        debug!("Handler received new handle");

        match handle.info() {
            Some(info) => {
                trace!("Adding new handle: {info:?}");
                self.handles.insert(info.id, handle.clone());
                send!(self.toplevel_tx, ToplevelEvent::New(handle));
            }
            None => {
                error!("Handle is missing information!");
            }
        }
    }

    fn update_handle(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        handle: ToplevelHandle,
    ) {
        debug!("Handler received handle update");

        match handle.info() {
            Some(info) => {
                trace!("Updating handle: {info:?}");
                self.handles.insert(info.id, handle.clone());
                send!(self.toplevel_tx, ToplevelEvent::Update(handle));
            }
            None => {
                error!("Handle is missing information!");
            }
        }
    }

    fn remove_handle(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        handle: ToplevelHandle,
    ) {
        debug!("Handler received handle close");
        match handle.info() {
            Some(info) => {
                self.handles.remove(&info.id);
                send!(self.toplevel_tx, ToplevelEvent::Remove(handle));
            }
            None => {
                error!("Handle is missing information!");
            }
        }
    }
}
