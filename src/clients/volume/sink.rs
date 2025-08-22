use super::{
    ArcMutVec, Client, ConnectionState, Event, percent_to_volume, scroll_to_volume,
    volume_to_percent,
};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::context::subscribe::Operation;
use libpulse_binding::def::SinkState;
use std::sync::{Arc, Mutex, mpsc};
use tokio::sync::broadcast;
use tracing::{debug, error, instrument, trace};

#[derive(Debug, Clone)]
pub struct Sink {
    index: u32,
    pub name: String,
    pub description: String,
    pub volume: f64,
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
            volume: volume_to_percent(value.volume),
            active: value.state == SinkState::Running,
        }
    }
}

impl Client {
    #[instrument(level = "trace")]
    pub fn sinks(&self) -> Arc<Mutex<Vec<Sink>>> {
        self.data.sinks.clone()
    }

    #[instrument(level = "trace")]
    pub fn set_default_sink(&self, name: &str) {
        if let ConnectionState::Connected { context, .. } = &*lock!(self.connection) {
            lock!(context).set_default_sink(name, |_| {});
        }
    }

    #[instrument(level = "trace")]
    pub fn set_default_volume(&self, value: f64) {
        trace!("raceived volume change: {value:?}");
        let mut active = None;
        for sync in &*lock!(self.data.sinks) {
            if sync.active {
                active = Some(sync.name.to_string());
                break;
            }
        }
        if let Some(name) = active {
            if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
                let (tx, rx) = mpsc::channel();

                introspector.get_sink_info_by_name(&name, move |info| {
                    let ListResult::Item(info) = info else {
                        return;
                    };

                    tx.send_expect(info.volume);
                });

                let mut volume = rx.recv().expect("to receive info");
                for v in volume.get_mut() {
                    let dval = v.0;
                    let val = scroll_to_volume(v.0, value);
                    trace!("changing value from: {dval:?} to: {val}");
                    v.0 = val;
                }

                introspector.set_sink_volume_by_name(&name, &volume, None);
            }
        }
    }

    #[instrument(level = "trace")]
    pub fn set_sink_volume(&self, name: &str, volume_percent: f64) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            let (tx, rx) = mpsc::channel();

            introspector.get_sink_info_by_name(name, move |info| {
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

            introspector.set_sink_volume_by_name(name, &volume, None);
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
