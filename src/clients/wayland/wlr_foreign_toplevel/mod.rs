use std::sync::RwLock;
use indexmap::IndexMap;
use tokio::sync::broadcast::Sender;
use tracing::trace;
use super::Env;
use handle::{ToplevelEvent, ToplevelChange, ToplevelInfo};
use manager::{ToplevelHandling, ToplevelStatusListener};
use wayland_client::DispatchData;
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1;
use crate::{send, write_lock};

pub mod handle;
pub mod manager;

impl ToplevelHandling for Env {
    fn listen<F>(&mut self, f: F) -> ToplevelStatusListener
    where
        F: FnMut(ZwlrForeignToplevelHandleV1, ToplevelEvent, DispatchData) + 'static,
    {
        self.toplevel.listen(f)
    }
}

pub fn update_toplevels(
    toplevels: &RwLock<IndexMap<usize, (ToplevelInfo, ZwlrForeignToplevelHandleV1)>>,
    handle: ZwlrForeignToplevelHandleV1,
    event: ToplevelEvent,
    tx: &Sender<ToplevelEvent>,
) {
    trace!("Received toplevel event: {:?}", event);

    if event.change == ToplevelChange::Close {
        write_lock!(toplevels).remove(&event.toplevel.id);
    } else {
        write_lock!(toplevels).insert(event.toplevel.id, (event.toplevel.clone(), handle));
    }

    send!(tx, event);
}
