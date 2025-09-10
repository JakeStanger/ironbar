use super::{ArcMutVec, Client, ConnectionState, Event, VolumeLevels};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::context::subscribe::Operation;
use libpulse_binding::def::SinkState;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, instrument, trace};

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

pub fn on_event(
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

                move |info| add(info, &sinks, &tx)
            });
        }
        Operation::Changed => {
            debug!("sink changed");
            introspect.get_sink_info_by_index(i, {
                let sinks = sinks.clone();
                let default_sink = default_sink.clone();
                let tx = tx.clone();

                move |info| update(info, &sinks, &default_sink, &tx)
            });
        }
        Operation::Removed => {
            debug!("sink removed");
            remove(i, sinks, tx);
        }
    }
}

pub fn add(info: ListResult<&SinkInfo>, sinks: &ArcMutVec<Sink>, tx: &broadcast::Sender<Event>) {
    let ListResult::Item(info) = info else {
        return;
    };

    trace!("adding {info:?}");

    lock!(sinks).push(info.into());
    tx.send_expect(Event::AddSink(info.into()));
}

fn update(
    info: ListResult<&SinkInfo>,
    sinks: &ArcMutVec<Sink>,
    default_sink: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
) {
    let ListResult::Item(info) = info else {
        return;
    };

    trace!("updating {info:?}");

    {
        let mut sinks = lock!(sinks);
        let Some(pos) = sinks.iter().position(|sink| sink.index == info.index) else {
            error!("received update to untracked sink input");
            return;
        };

        sinks[pos] = info.into();

        // update in local copy
        if !sinks[pos].active
            && let Some(default_sink) = &*lock!(default_sink)
        {
            sinks[pos].active = &sinks[pos].name == default_sink;
        }
    }

    let mut sink: Sink = info.into();

    // update in broadcast copy
    if !sink.active
        && let Some(default_sink) = &*lock!(default_sink)
    {
        sink.active = &sink.name == default_sink;
    }

    tx.send_expect(Event::UpdateSink(sink));
}

fn remove(index: u32, sinks: &ArcMutVec<Sink>, tx: &broadcast::Sender<Event>) {
    trace!("removing {index}");

    let mut sinks = lock!(sinks);

    if let Some(pos) = sinks.iter().position(|s| s.index == index) {
        let info = sinks.remove(pos);
        tx.send_expect(Event::RemoveSink(info.name));
    }
}
