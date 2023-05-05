use super::Environment;
use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use tracing::debug;
use wayland_client::protocol::wl_output;
use wayland_client::{Connection, QueueHandle};

impl Environment {
    pub fn output_info(&mut self) -> Vec<OutputInfo> {
        self.output_state
            .outputs()
            .filter_map(|output| self.output_state.info(&output))
            .collect()
    }
}

// In order to use OutputDelegate, we must implement this trait to indicate when something has happened to an
// output and to provide an instance of the output state to the delegate when dispatching events.
impl OutputHandler for Environment {
    // First we need to provide a way to access the delegate.
    //
    // This is needed because delegate implementations for handling events use the application data type in
    // their function signatures. This allows the implementation to access an instance of the type.
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    // Then there exist these functions that indicate the lifecycle of an output.
    // These will be called as appropriate by the delegate implementation.

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        debug!("Handler received new output");
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        debug!("Handle received output destruction");
    }
}
