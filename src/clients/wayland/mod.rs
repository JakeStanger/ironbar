mod client;

mod wlr_foreign_toplevel;

use std::collections::HashMap;
use async_once::AsyncOnce;
use lazy_static::lazy_static;
use std::fmt::Debug;
use cfg_if::cfg_if;
use smithay_client_toolkit::default_environment;
use smithay_client_toolkit::environment::Environment;
use smithay_client_toolkit::reexports::calloop::RegistrationToken;
use wayland_client::{Attached, Interface};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;
pub use wlr_foreign_toplevel::handle::{ToplevelChange, ToplevelEvent, ToplevelInfo};
use wlr_foreign_toplevel::manager::{ToplevelHandler};

pub use client::WaylandClient;

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        mod wlr_data_control;

        use wayland_protocols::wlr::unstable::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
        use wlr_data_control::manager::DataControlDeviceHandler;
        pub use wlr_data_control::{ClipboardItem, ClipboardValue};
    }
}

/// A utility for lazy-loading globals.
/// Taken from `smithay_client_toolkit` where it's not exposed
#[derive(Debug)]
enum LazyGlobal<I: Interface> {
    Unknown,
    Seen { id: u32, version: u32 },
    Bound(Attached<I>),
}

pub struct DData {
    env: Environment<Env>,
    offer_tokens: HashMap<u128, RegistrationToken>,
}

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        default_environment!(Env,
            fields = [
                toplevel: ToplevelHandler,
                data_control_device: DataControlDeviceHandler
            ],
            singles = [
                ZwlrForeignToplevelManagerV1 => toplevel,
                ZwlrDataControlManagerV1 => data_control_device
            ],
        );
    } else {
            default_environment!(Env,
            fields = [
                toplevel: ToplevelHandler,
            ],
            singles = [
                ZwlrForeignToplevelManagerV1 => toplevel,
            ],
        );
    }
}

lazy_static! {
    static ref CLIENT: AsyncOnce<WaylandClient> =
        AsyncOnce::new(async { WaylandClient::new().await });
}

pub async fn get_client() -> &'static WaylandClient {
    CLIENT.get().await
}
