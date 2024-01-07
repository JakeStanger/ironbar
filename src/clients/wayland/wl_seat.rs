use super::Environment;
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use tracing::debug;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, QueueHandle};

impl Environment {
    /// Gets the default seat.
    pub(crate) fn default_seat(&self) -> WlSeat {
        self.seat_state.seats().next().expect("one seat to exist")
    }
}

impl SeatHandler for Environment {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _seat: WlSeat) {
        debug!("Handler received new seat");
    }

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        _: Capability,
    ) {
        debug!("Handler received new capability");

        #[cfg(feature = "clipboard")]
        if !self
            .data_control_devices
            .iter_mut()
            .any(|entry| entry.seat == seat)
        {
            debug!("Adding new data control device");
            // create the data device here for this seat
            let data_control_device_manager = &self.data_control_device_manager_state;
            let data_control_device = data_control_device_manager.get_data_device(qh, &seat);
            self.data_control_devices
                .push(super::DataControlDeviceEntry {
                    seat: seat.clone(),
                    device: data_control_device,
                });
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        seat: WlSeat,
        _: Capability,
    ) {
        debug!("Handler received capability removal");

        #[cfg(feature = "clipboard")]
        self.data_control_devices.retain(|entry| entry.seat != seat);
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _seat: WlSeat) {
        debug!("Handler received seat removal");
    }
}
