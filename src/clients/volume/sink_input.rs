use super::{percent_to_volume, volume_to_percent, ArcMutVec, Client, ConnectionState, Event};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::SinkInputInfo;
use libpulse_binding::context::subscribe::Operation;
use libpulse_binding::context::Context;
use std::sync::{mpsc, Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct SinkInput {
    pub index: u32,
    pub name: String,
    pub volume: f64,
    pub muted: bool,

    pub can_set_volume: bool,
}

impl From<&SinkInputInfo<'_>> for SinkInput {
    fn from(value: &SinkInputInfo) -> Self {
        Self {
            index: value.index,
            name: value
                .name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            muted: value.mute,
            volume: volume_to_percent(value.volume),
            can_set_volume: value.has_volume && value.volume_writable,
        }
    }
}

impl Client {
    pub fn sink_inputs(&self) -> Arc<Mutex<Vec<SinkInput>>> {
        self.data.sink_inputs.clone()
    }

    pub fn set_input_volume(&self, index: u32, volume_percent: f64) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            let (tx, rx) = mpsc::channel();

            introspector.get_sink_input_info(index, move |info| {
                let ListResult::Item(info) = info else {
                    return;
                };
                tx.send_expect(info.volume);
            });

            let new_volume = percent_to_volume(volume_percent);

            let mut volume = rx.recv().expect("to receive info");
            for v in volume.get_mut() {
                v.0 = new_volume;
            }

            introspector.set_sink_input_volume(index, &volume, None);
        }
    }

    pub fn set_input_muted(&self, index: u32, muted: bool) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            introspector.set_sink_input_mute(index, muted, None);
        }
    }
}

pub fn on_event(
    context: &Arc<Mutex<Context>>,
    inputs: &ArcMutVec<SinkInput>,
    tx: &broadcast::Sender<Event>,
    op: Operation,
    i: u32,
) {
    let introspect = lock!(context).introspect();

    match op {
        Operation::New => {
            debug!("new sink input");
            introspect.get_sink_input_info(i, {
                let inputs = inputs.clone();
                let tx = tx.clone();

                move |info| add(info, &inputs, &tx)
            });
        }
        Operation::Changed => {
            debug!("sink input changed");
            introspect.get_sink_input_info(i, {
                let inputs = inputs.clone();
                let tx = tx.clone();

                move |info| update(info, &inputs, &tx)
            });
        }
        Operation::Removed => {
            debug!("sink input removed");
            remove(i, inputs, tx);
        }
    }
}

pub fn add(
    info: ListResult<&SinkInputInfo>,
    inputs: &ArcMutVec<SinkInput>,
    tx: &broadcast::Sender<Event>,
) {
    let ListResult::Item(info) = info else {
        return;
    };

    lock!(inputs).push(info.into());
    tx.send_expect(Event::AddInput(info.into()));
}

fn update(
    info: ListResult<&SinkInputInfo>,
    inputs: &ArcMutVec<SinkInput>,
    tx: &broadcast::Sender<Event>,
) {
    let ListResult::Item(info) = info else {
        return;
    };

    {
        let mut inputs = lock!(inputs);
        let Some(pos) = inputs.iter().position(|input| input.index == info.index) else {
            error!("received update to untracked sink input");
            return;
        };

        inputs[pos] = info.into();
    }

    tx.send_expect(Event::UpdateInput(info.into()));
}

fn remove(index: u32, inputs: &ArcMutVec<SinkInput>, tx: &broadcast::Sender<Event>) {
    let mut inputs = lock!(inputs);

    if let Some(pos) = inputs.iter().position(|s| s.index == index) {
        let info = inputs.remove(pos);
        tx.send_expect(Event::RemoveInput(info.index));
    }
}
