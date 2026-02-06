use std::sync::{Arc, Mutex};

use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::context::subscribe::Operation;
use libpulse_binding::def::SinkState;
use tokio::sync::broadcast;
use tracing::{debug, instrument};

use super::{ArcMutVec, Client, ConnectionState, Event, HasIndex, PulseObject, VolumeLevels};
use crate::lock;

#[derive(Debug, Clone)]
pub struct Sink {
    index: u32,
    pub name: String,
    pub description: String,
    pub volume: VolumeLevels,
    pub muted: bool,
    pub active: bool,
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
            active: value.state == SinkState::Running,
        }
    }
}

impl<'a> HasIndex for SinkInfo<'a> {
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
    fn name(&self) -> String {
        self.name.clone()
    }
    #[inline]
    fn active(&self) -> bool {
        self.active
    }
    #[inline]
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
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
    pub fn set_default_sink(&self, name: &str) {
        if let ConnectionState::Connected { context, .. } = &*lock!(self.connection) {
            lock!(context).set_default_sink(name, |_| {});
        }
    }

    #[instrument(level = "trace")]
    pub fn set_sink_volume(&self, name: &str, volume: f64) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
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
            introspector.set_sink_volume_by_name(name, &volume_levels.into(), None);
        }
    }

    #[instrument(level = "trace")]
    pub fn set_sink_muted(&self, name: &str, muted: bool) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            introspector.set_sink_mute_by_name(name, muted, None);
        }
    }
}

impl Sink {
    pub(super) fn on_event(
        context: &Arc<Mutex<Context>>,
        sinks: &ArcMutVec<Sink>,
        default_sink: &Arc<Mutex<Option<String>>>,
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
                    let default_sink = default_sink.clone();
                    let tx = tx.clone();

                    move |info| Self::update(info, &sinks, Some(&default_sink), &tx)
                });
            }
            Operation::Removed => {
                debug!("sink removed");
                Self::remove(i, sinks, tx);
            }
        }
    }
}
