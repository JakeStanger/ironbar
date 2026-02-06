use std::sync::{Arc, Mutex};

use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SourceInfo;
use libpulse_binding::context::subscribe::Operation;
use libpulse_binding::def::SourceState;
use tokio::sync::broadcast;
use tracing::{debug, instrument};

use super::{ArcMutVec, Client, ConnectionState, Event, HasIndex, PulseObject, VolumeLevels};
use crate::lock;

#[derive(Debug, Clone)]
pub struct Source {
    index: u32,
    pub name: String,
    pub description: String,
    pub volume: VolumeLevels,
    pub muted: bool,
    pub active: bool,
    pub monitor: bool,
}

impl From<&SourceInfo<'_>> for Source {
    fn from(value: &SourceInfo<'_>) -> Self {
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
            active: value.state == SourceState::Running,
            monitor: value.monitor_of_sink.is_some(),
        }
    }
}

impl<'a> HasIndex for SourceInfo<'a> {
    fn index(&self) -> u32 {
        self.index
    }
}

impl HasIndex for Source {
    fn index(&self) -> u32 {
        self.index
    }
}

impl<'a> PulseObject<'a> for Source {
    type Inner = SourceInfo<'a>;

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
        Event::AddSource(info)
    }
    #[inline]
    fn update_event(info: Self) -> Event {
        Event::UpdateSource(info)
    }
    #[inline]
    fn remove_event(info: Self) -> Event {
        Event::RemoveSource(info.name)
    }
}

impl Client {
    #[instrument(level = "trace")]
    pub fn sources(&self) -> ArcMutVec<Source> {
        self.data.sources.clone()
    }

    #[instrument(level = "trace")]
    pub fn set_default_source(&self, name: &str) {
        if let ConnectionState::Connected { context, .. } = &*lock!(self.connection) {
            lock!(context).set_default_source(name, |_| {});
        }
    }

    #[instrument(level = "trace")]
    pub fn set_source_volume(&self, name: &str, volume: f64) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            let Some(mut volume_levels) = ({
                let sources = self.sources();
                lock!(sources).iter().find_map(|s| {
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
            introspector.set_source_volume_by_name(name, &volume_levels.into(), None);
        }
    }

    #[instrument(level = "trace")]
    pub fn set_source_muted(&self, name: &str, muted: bool) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            introspector.set_source_mute_by_name(name, muted, None);
        }
    }
}

impl Source {
    pub(super) fn on_event(
        context: &Arc<Mutex<Context>>,
        sources: &ArcMutVec<Source>,
        default_source: &Arc<Mutex<Option<String>>>,
        tx: &broadcast::Sender<Event>,
        op: Operation,
        i: u32,
    ) {
        let introspect = lock!(context).introspect();

        match op {
            Operation::New => {
                debug!("new source");
                introspect.get_source_info_by_index(i, {
                    let sources = sources.clone();
                    let tx = tx.clone();

                    move |info| Self::add(info, &sources, &tx)
                });
            }
            Operation::Changed => {
                debug!("source changed");
                introspect.get_source_info_by_index(i, {
                    let source = sources.clone();
                    let default_source = default_source.clone();
                    let tx = tx.clone();

                    move |info| Self::update(info, &source, Some(&default_source), &tx)
                });
            }
            Operation::Removed => {
                debug!("source removed");
                Self::remove(i, sources, tx);
            }
        }
    }
}
