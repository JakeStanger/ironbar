use std::sync::{Arc, Mutex};

use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInputInfo;
use libpulse_binding::context::subscribe::Operation;
use tokio::sync::broadcast;
use tracing::{debug, instrument};

use super::{ArcMutVec, Client, ConnectionState, Event, HasIndex, PulseObject, VolumeLevels};
use crate::lock;

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

impl<'a> HasIndex for SinkInputInfo<'a> {
    fn index(&self) -> u32 {
        self.index
    }
}

impl HasIndex for SinkInput {
    fn index(&self) -> u32 {
        self.index
    }
}

impl<'a> PulseObject<'a> for SinkInput {
    type Inner = SinkInputInfo<'a>;

    #[inline]
    fn name(&self) -> String {
        self.name.clone()
    }
    #[inline]
    fn active(&self) -> bool {
        true
    }
    #[inline]
    fn set_active(&mut self, _active: bool) {}

    #[inline]
    fn add_event(info: Self) -> Event {
        Event::AddInput(info)
    }
    #[inline]
    fn update_event(info: Self) -> Event {
        Event::UpdateInput(info)
    }
    #[inline]
    fn remove_event(info: Self) -> Event {
        Event::RemoveInput(info.index)
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

impl SinkInput {
    pub(super) fn on_event(
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

                    move |info| Self::add(info, &inputs, &tx)
                });
            }
            Operation::Changed => {
                debug!("sink input changed");
                introspect.get_sink_input_info(i, {
                    let inputs = inputs.clone();
                    let tx = tx.clone();

                    move |info| Self::update(info, &inputs, None, &tx)
                });
            }
            Operation::Removed => {
                debug!("sink input removed");
                Self::remove(i, inputs, tx);
            }
        }
    }
}
