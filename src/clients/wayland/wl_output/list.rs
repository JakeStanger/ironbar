//! Obtain the vector of Outputs

use std::error::Error;

use smithay_client_toolkit::{
    delegate_output, delegate_registry,
    output::{OutputHandler, OutputInfo, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{globals::registry_queue_init, protocol::wl_output::{self}, Connection, QueueHandle};

pub fn get() -> Result<Vec<OutputInfo>, Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);

    let output_delegate = OutputState::new(&globals, &qh);

    let mut list_outputs = ListOutputs { registry_state, output_state: output_delegate };

    event_queue.roundtrip(&mut list_outputs)?;

    let mut outputs = vec![];
    for output in list_outputs.output_state.outputs() {
        outputs.push(list_outputs
                .output_state
                .info(&output)
                .ok_or_else(|| "output has no info".to_owned())?);
    }

    return Ok(outputs);
}

struct ListOutputs {
    registry_state: RegistryState,
    output_state: OutputState,
}

impl OutputHandler for ListOutputs {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
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
    }
}

delegate_output!(ListOutputs);
delegate_registry!(ListOutputs);

impl ProvidesRegistryState for ListOutputs {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers! {
        OutputState,
    }
}
