use super::{Client, Environment, Event};
use crate::channels::AsyncSenderExt;
use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use tokio::sync::broadcast;
use tracing::{debug, error};
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, QueueHandle};

#[derive(Debug, Clone)]
pub struct OutputEvent {
    pub output: OutputInfo,
    pub event_type: OutputEventType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputEventType {
    New,
    Update,
    Destroyed,
}

impl Client {
    /// Gets the information for all outputs.
    #[cfg(feature = "ipc")]
    pub fn output_info_all(&self) -> Vec<OutputInfo> {
        use super::{Request, Response};
        match self.send_request(Request::OutputInfoAll) {
            Response::OutputInfoAll(info) => info,
            _ => unreachable!(),
        }
    }

    /// Subscribes to events from outputs.
    pub fn subscribe_outputs(&self) -> broadcast::Receiver<OutputEvent> {
        self.output_channel.0.subscribe()
    }
}

impl Environment {
    #[cfg(feature = "ipc")]
    pub fn output_info_all(&mut self) -> Vec<OutputInfo> {
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

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        debug!("Handler received new output");
        if let Some(info) = self.output_state.info(&output) {
            self.event_tx.send_spawn(Event::Output(OutputEvent {
                output: info,
                event_type: OutputEventType::New,
            }));
        } else {
            error!("Output is missing information!");
        }
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        debug!("Handle received output update");
        if let Some(info) = self.output_state.info(&output) {
            self.event_tx.send_spawn(Event::Output(OutputEvent {
                output: info,
                event_type: OutputEventType::Update,
            }));
        } else {
            error!("Output is missing information!");
        }
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        debug!("Handle received output destruction");
        if let Some(info) = self.output_state.info(&output) {
            self.event_tx.send_spawn(Event::Output(OutputEvent {
                output: info,
                event_type: OutputEventType::Destroyed,
            }));
        } else {
            error!("Output is missing information!");
        }
    }
}
