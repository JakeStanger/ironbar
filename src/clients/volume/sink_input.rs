use super::{ArcMutVec, Client, ConnectionState, Event, VolumeLevels};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInputInfo;
use libpulse_binding::context::subscribe::Operation;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, instrument, trace};

#[derive(Debug, Clone)]
pub struct SinkInput {
    pub index: u32,
    pub name: String,
    pub volume: VolumeLevels,
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
            volume: value.volume.into(),
            can_set_volume: value.has_volume && value.volume_writable,
        }
    }
}

impl Client {
    #[instrument(level = "trace")]
    pub fn sink_inputs(&self) -> ArcMutVec<SinkInput> {
        self.data.sink_inputs.clone()
    }

    #[instrument(level = "trace")]
    pub fn set_input_volume(&self, index: u32, volume_percent: f64) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            let Some(mut volume_levels) = ({
                let inputs = self.sink_inputs();
                lock!(inputs).iter().find_map(|s| {
                    if s.index == index {
                        Some(s.volume.clone())
                    } else {
                        None
                    }
                })
            }) else {
                return;
            };

            volume_levels.set_percent(volume_percent);
            introspector.set_sink_input_volume(index, &volume_levels.into(), None);
        }
    }

    #[instrument(level = "trace")]
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

    trace!("adding {info:?}");

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

    trace!("updating {info:?}");

    let input_info: SinkInput = info.into();

    {
        let mut inputs = lock!(inputs);
        if let Some(pos) = inputs
            .iter()
            .position(|input| input.index == input_info.index)
        {
            inputs[pos] = input_info.clone();
        } else {
            error!("received update to untracked sink input");
            return;
        }
    }

    tx.send_expect(Event::UpdateInput(input_info));
}

fn remove(index: u32, inputs: &ArcMutVec<SinkInput>, tx: &broadcast::Sender<Event>) {
    let mut inputs = lock!(inputs);

    trace!("removing {index}");

    if let Some(pos) = inputs.iter().position(|s| s.index == index) {
        let info = inputs.remove(pos);
        tx.send_expect(Event::RemoveInput(info.index));
    }
}
