mod client;
mod toplevel;
mod toplevel_manager;

extern crate smithay_client_toolkit as sctk;

use self::toplevel_manager::ToplevelHandler;
pub use crate::wayland::toplevel::{ToplevelChange, ToplevelEvent, ToplevelInfo};
use crate::wayland::toplevel_manager::{ToplevelHandling, ToplevelStatusListener};
use async_once::AsyncOnce;
use lazy_static::lazy_static;
use wayland_client::{Attached, DispatchData, Interface};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
};

pub use client::WaylandClient;

/// A utility for lazy-loading globals.
/// Taken from smithay_client_toolkit where it's not exposed
#[derive(Debug)]
enum LazyGlobal<I: Interface> {
    Unknown,
    Seen { id: u32, version: u32 },
    Bound(Attached<I>),
}

sctk::default_environment!(Env,
    fields = [
        toplevel: ToplevelHandler
    ],
    singles = [
        ZwlrForeignToplevelManagerV1 => toplevel
    ],
);

impl ToplevelHandling for Env {
    fn listen<F>(&mut self, f: F) -> ToplevelStatusListener
    where
        F: FnMut(ZwlrForeignToplevelHandleV1, ToplevelEvent, DispatchData) + 'static,
    {
        self.toplevel.listen(f)
    }
}

lazy_static! {
    static ref CLIENT: AsyncOnce<WaylandClient> =
        AsyncOnce::new(async { WaylandClient::new().await });
}

pub async fn get_client() -> &'static WaylandClient {
    CLIENT.get().await
}
