pub mod handle;
pub mod manager;

use self::handle::ToplevelHandleHandler;
use self::manager::ToplevelManagerHandler;
use super::{Client, Environment, Event, Request, Response};
use tokio::sync::broadcast;
use tracing::{debug, error, trace};
use wayland_client::{Connection, QueueHandle};

use crate::channels::AsyncSenderExt;
pub use handle::{ToplevelHandle, ToplevelInfo};

#[derive(Debug, Clone)]
pub enum ToplevelEvent {
    New(ToplevelInfo),
    Update(ToplevelInfo),
    Remove(ToplevelInfo),
}

impl Client {
    /// Gets the information for all currently open toplevels (windows)
    pub fn toplevel_info_all(&self) -> Vec<ToplevelInfo> {
        match self.send_request(Request::ToplevelInfoAll) {
            Response::ToplevelInfoAll(infos) => infos,
            _ => unreachable!(),
        }
    }

    /// Focuses the toplevel with the provided ID.
    #[cfg(feature = "launcher")]
    pub fn toplevel_focus(&self, handle_id: usize) {
        match self.send_request(Request::ToplevelFocus(handle_id)) {
            Response::Ok => (),
            _ => unreachable!(),
        }
    }

    /// Minimizes the toplevel with the provided ID.
    #[cfg(feature = "launcher")]
    pub fn toplevel_minimize(&self, handle_id: usize) {
        match self.send_request(Request::ToplevelMinimize(handle_id)) {
            Response::Ok => (),
            _ => unreachable!(),
        }
    }

    /// Subscribes to events from toplevels.
    pub fn subscribe_toplevels(&self) -> broadcast::Receiver<ToplevelEvent> {
        self.toplevel_channel.0.subscribe()
    }
}

impl ToplevelManagerHandler for Environment {
    fn toplevel(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {
        debug!("Manager received new handle");
    }
}

impl ToplevelHandleHandler for Environment {
    fn new_handle(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, handle: ToplevelHandle) {
        debug!("Handler received new handle");

        match handle.info() {
            Some(info) => {
                if info.app_id.is_empty() {
                    trace!("ignoring xwayland dialog");
                    return;
                }

                trace!("Adding new handle: {info:?}");
                self.handles.push(handle.clone());
                if let Some(info) = handle.info() {
                    self.event_tx
                        .send_spawn(Event::Toplevel(ToplevelEvent::New(info)));
                }
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
        trace!("Handler received handle update");

        match handle.info() {
            Some(info) => {
                trace!("Updating handle: {info:?}");
                if let Some(info) = handle.info() {
                    self.event_tx
                        .send_spawn(Event::Toplevel(ToplevelEvent::Update(info)));
                }
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

        self.handles.retain(|h| h != &handle);
        if let Some(info) = handle.info() {
            self.event_tx
                .send_spawn(Event::Toplevel(ToplevelEvent::Remove(info)));
        }
    }
}
