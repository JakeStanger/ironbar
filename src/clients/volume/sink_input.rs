use super::{ArcMutVec, Client, Event, HasIndex, PulseObject, Request, VolumeLevels};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInputInfo;
use libpulse_binding::context::subscribe::Operation;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, instrument};

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
        let mut name = value.name.as_ref().map(ToString::to_string);

        if name.is_none()
            || name.as_ref().is_some_and(|s| s.starts_with("audio stream"))
            || name
                .as_ref()
                .is_some_and(|s| s.starts_with("Playback Stream"))
        {
            name = value
                .proplist
                .get_str("application.name")
                .or_else(|| value.proplist.get_str("application.process.binary"))
                .or_else(|| value.proplist.get_str("node.name"));
        }

        Self {
            index: value.index,
            name: name.unwrap_or_else(|| format!("input {}", value.index)),
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

        self.req_tx
            .send_expect(Request::SinkInputVolume(index, volume_levels));
    }

    #[instrument(level = "trace")]
    pub fn set_input_muted(&self, index: u32, muted: bool) {
        self.req_tx
            .send_expect(Request::SinkInputMuted(index, muted));
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
