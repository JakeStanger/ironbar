mod client;

mod macros;
mod wl_output;
mod wl_seat;
mod wlr_foreign_toplevel;

use self::wlr_foreign_toplevel::manager::ToplevelManagerState;
use crate::{arc_mut, delegate_foreign_toplevel_handle, delegate_foreign_toplevel_manager};
use cfg_if::cfg_if;
use lazy_static::lazy_static;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::LoopHandle;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::{
    delegate_output, delegate_registry, delegate_seat, registry_handlers,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use wayland_client::protocol::wl_seat::WlSeat;

pub use self::client::WaylandClient;
pub use self::wlr_foreign_toplevel::{ToplevelEvent, ToplevelHandle, ToplevelInfo};

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        mod wlr_data_control;

        use crate::{delegate_data_control_device, delegate_data_control_device_manager, delegate_data_control_offer, delegate_data_control_source};
        use self::wlr_data_control::device::DataControlDevice;
        use self::wlr_data_control::manager::DataControlDeviceManagerState;
        use self::wlr_data_control::source::CopyPasteSource;
        use self::wlr_data_control::SelectionOfferItem;

        pub use wlr_data_control::{ClipboardItem, ClipboardValue};

        pub struct DataControlDeviceEntry {
            seat: WlSeat,
            device: DataControlDevice,
        }
    }
}

pub struct Environment {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
    pub foreign_toplevel_manager_state: ToplevelManagerState,
    #[cfg(feature = "clipboard")]
    pub data_control_device_manager_state: DataControlDeviceManagerState,
    pub loop_handle: LoopHandle<'static, Self>,

    pub seats: Vec<WlSeat>,

    #[cfg(feature = "clipboard")]
    pub data_control_devices: Vec<DataControlDeviceEntry>,
    #[cfg(feature = "clipboard")]
    pub selection_offers: Vec<SelectionOfferItem>,
    #[cfg(feature = "clipboard")]
    pub copy_paste_sources: Vec<CopyPasteSource>,

    pub handles: HashMap<usize, ToplevelHandle>,
    #[cfg(feature = "clipboard")]
    clipboard: Arc<Mutex<Option<Arc<ClipboardItem>>>>,

    toplevel_tx: broadcast::Sender<ToplevelEvent>,
    #[cfg(feature = "clipboard")]
    clipboard_tx: broadcast::Sender<Arc<ClipboardItem>>,
}

// Now we need to say we are delegating the responsibility of output related events for our application data
// type to the requisite delegate.
delegate_output!(Environment);
delegate_seat!(Environment);

delegate_foreign_toplevel_manager!(Environment);
delegate_foreign_toplevel_handle!(Environment);

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        delegate_data_control_device_manager!(Environment);
        delegate_data_control_device!(Environment);
        delegate_data_control_source!(Environment);
        delegate_data_control_offer!(Environment);
    }
}

// In order for our delegate to know of the existence of globals, we need to implement registry
// handling for the program. This trait will forward events to the RegistryHandler trait
// implementations.
delegate_registry!(Environment);

// In order for delegate_registry to work, our application data type needs to provide a way for the
// implementation to access the registry state.
//
// We also need to indicate which delegates will get told about globals being created. We specify
// the types of the delegates inside the array.
impl ProvidesRegistryState for Environment {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

lazy_static! {
    static ref CLIENT: Arc<Mutex<WaylandClient>> = arc_mut!(WaylandClient::new());
}

pub fn get_client() -> Arc<Mutex<WaylandClient>> {
    CLIENT.clone()
}
