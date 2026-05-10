use super::{ArcMutVec, Client, Event, HasIndex, PulseObject, Request, VolumeLevels};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::context::subscribe::Operation;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, instrument};

#[derive(Debug, Clone)]
pub struct Sink {
    index: u32,
    pub name: String,
    pub description: String,
    pub volume: VolumeLevels,
    pub muted: bool,
}

impl From<&SinkInfo<'_>> for Sink {
    fn from(value: &SinkInfo) -> Self {
        Self {
            index: value.index,
            name: value
                .name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            description: value
                .description
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            muted: value.mute,
            volume: value.volume.into(),
        }
    }
}

impl HasIndex for SinkInfo<'_> {
    fn index(&self) -> u32 {
        self.index
    }
}

impl HasIndex for Sink {
    fn index(&self) -> u32 {
        self.index
    }
}

impl<'a> PulseObject<'a> for Sink {
    type Inner = SinkInfo<'a>;

    #[inline]
    fn add_event(info: Self) -> Event {
        Event::AddSink(info)
    }
    #[inline]
    fn update_event(info: Self) -> Event {
        Event::UpdateSink(info)
    }
    #[inline]
    fn remove_event(info: Self) -> Event {
        Event::RemoveSink(info.name)
    }
}

impl Client {
    #[instrument(level = "trace")]
    pub fn sinks(&self) -> ArcMutVec<Sink> {
        self.data.sinks.clone()
    }

    #[instrument(level = "trace")]
    pub fn default_sink(&self) -> Option<String> {
        lock!(self.data.default_sink_name).clone()
    }

    #[instrument(level = "trace")]
    pub fn set_default_sink(&self, name: &str) {
        self.req_tx
            .send_expect(Request::SinkDefault(name.to_string()));
    }

    #[instrument(level = "trace")]
    pub fn set_sink_volume(&self, name: &str, volume: f64) {
        let Some(mut volume_levels) = ({
            let sinks = self.sinks();
            lock!(sinks).iter().find_map(|s| {
                if s.name == name {
                    Some(s.volume.clone())
                } else {
                    None
                }
            })
        }) else {
            return;
        };

        volume_levels.set_percent(volume);

        self.req_tx
            .send_expect(Request::SinkVolume(name.to_string(), volume_levels));
    }

    #[instrument(level = "trace")]
    pub fn set_sink_muted(&self, name: &str, muted: bool) {
        self.req_tx
            .send_expect(Request::SinkMuted(name.to_string(), muted));
    }
}

impl Sink {
    pub(super) fn on_event(
        context: &Arc<Mutex<Context>>,
        sinks: &ArcMutVec<Sink>,
        tx: &broadcast::Sender<Event>,
        op: Operation,
        i: u32,
    ) {
        let introspect = lock!(context).introspect();

        match op {
            Operation::New => {
                debug!("new sink");
                introspect.get_sink_info_by_index(i, {
                    let sinks = sinks.clone();
                    let tx = tx.clone();

                    move |info| Self::add(info, &sinks, &tx)
                });
            }
            Operation::Changed => {
                debug!("sink changed");
                introspect.get_sink_info_by_index(i, {
                    let sinks = sinks.clone();
                    let tx = tx.clone();

                    move |info| {
                        Self::update(info, &sinks, &tx);
                    }
                });
            }
            Operation::Removed => {
                debug!("sink removed");
                Self::remove(i, sinks, tx);
            }
        }
    }
}
