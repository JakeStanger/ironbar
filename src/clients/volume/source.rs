use std::sync::{Arc, Mutex};

use super::{ArcMutVec, Client, Event, HasIndex, PulseObject, Request, VolumeLevels};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SourceInfo;
use libpulse_binding::context::subscribe::Operation;
use tokio::sync::broadcast;
use tracing::{debug, instrument};

#[derive(Debug, Clone)]
pub struct Source {
    index: u32,
    pub name: String,
    pub description: String,
    pub volume: VolumeLevels,
    pub muted: bool,
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
            monitor: value.monitor_of_sink.is_some(),
        }
    }
}

impl HasIndex for SourceInfo<'_> {
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
    pub fn default_source(&self) -> Option<String> {
        lock!(self.data.default_source_name).clone()
    }

    #[instrument(level = "trace")]
    pub fn set_default_source(&self, name: &str) {
        self.req_tx
            .send_expect(Request::SourceDefault(name.to_string()));
    }

    #[instrument(level = "trace")]
    pub fn set_source_volume(&self, name: &str, volume: f64) {
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

        self.req_tx
            .send_expect(Request::SourceVolume(name.to_string(), volume_levels));
    }

    #[instrument(level = "trace")]
    pub fn set_source_muted(&self, name: &str, muted: bool) {
        self.req_tx
            .send_expect(Request::SourceMuted(name.to_string(), muted));
    }
}

impl Source {
    pub(super) fn on_event(
        context: &Arc<Mutex<Context>>,
        sources: &ArcMutVec<Source>,
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
                    let tx = tx.clone();

                    move |info| Self::update(info, &source, &tx)
                });
            }
            Operation::Removed => {
                debug!("source removed");
                Self::remove(i, sources, tx);
            }
        }
    }
}
