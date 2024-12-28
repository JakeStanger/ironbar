mod sink;
mod sink_input;

use crate::{arc_mut, lock, register_client, spawn_blocking, APP_ID};
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::{Introspector, ServerInfo};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context, FlagSet, State};
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::proplist::Proplist;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::channels::SyncSenderExt;
pub use sink::Sink;
pub use sink_input::SinkInput;

type ArcMutVec<T> = Arc<Mutex<Vec<T>>>;

#[derive(Debug, Clone)]
pub enum Event {
    AddSink(Sink),
    UpdateSink(Sink),
    RemoveSink(String),

    AddInput(SinkInput),
    UpdateInput(SinkInput),
    RemoveInput(u32),
}

#[derive(Debug)]
pub struct Client {
    connection: Arc<Mutex<ConnectionState>>,

    data: Data,

    tx: broadcast::Sender<Event>,
    _rx: broadcast::Receiver<Event>,
}

#[derive(Debug, Default, Clone)]
struct Data {
    sinks: ArcMutVec<Sink>,
    sink_inputs: ArcMutVec<SinkInput>,

    default_sink_name: Arc<Mutex<Option<String>>>,
}

pub enum ConnectionState {
    Disconnected,
    Connected {
        context: Arc<Mutex<Context>>,
        introspector: Introspector,
    },
}

impl Debug for ConnectionState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Disconnected => "Disconnected",
                Self::Connected { .. } => "Connected",
            }
        )
    }
}

impl Client {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel(32);

        Self {
            connection: arc_mut!(ConnectionState::Disconnected),
            data: Data::default(),
            tx,
            _rx: rx,
        }
    }

    /// Starts the client.
    fn run(&self) {
        let Some(mut proplist) = Proplist::new() else {
            error!("Failed to create PA proplist");
            return;
        };

        if proplist.set_str("APPLICATION_NAME", APP_ID).is_err() {
            error!("Failed to update PA proplist");
        }

        let Some(mut mainloop) = Mainloop::new() else {
            error!("Failed to create PA mainloop");
            return;
        };

        let Some(context) = Context::new_with_proplist(&mainloop, "Ironbar Context", &proplist)
        else {
            error!("Failed to create PA context");
            return;
        };

        let context = arc_mut!(context);

        let state_callback = Box::new({
            let context = context.clone();
            let data = self.data.clone();
            let tx = self.tx.clone();

            move || on_state_change(&context, &data, &tx)
        });

        lock!(context).set_state_callback(Some(state_callback));

        if let Err(err) = lock!(context).connect(None, FlagSet::NOAUTOSPAWN, None) {
            error!("{err:?}");
        }

        let introspector = lock!(context).introspect();

        {
            let mut inner = lock!(self.connection);
            *inner = ConnectionState::Connected {
                context,
                introspector,
            };
        }

        loop {
            match mainloop.iterate(true) {
                IterateResult::Success(_) => {}
                IterateResult::Err(err) => error!("{err:?}"),
                IterateResult::Quit(_) => break,
            }
        }
    }

    /// Gets an event receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

/// Creates a new Pulse volume client.
pub fn create_client() -> Arc<Client> {
    let client = Arc::new(Client::new());

    {
        let client = client.clone();
        spawn_blocking(move || {
            client.run();
        });
    }

    client
}

fn on_state_change(context: &Arc<Mutex<Context>>, data: &Data, tx: &broadcast::Sender<Event>) {
    let Ok(state) = context.try_lock().map(|lock| lock.get_state()) else {
        return;
    };

    match state {
        State::Ready => {
            info!("connected to server");

            let introspect = lock!(context).introspect();
            let introspect2 = lock!(context).introspect();

            introspect.get_sink_info_list({
                let sinks = data.sinks.clone();
                let default_sink = data.default_sink_name.clone();

                let tx = tx.clone();

                move |info| match info {
                    ListResult::Item(_) => sink::add(info, &sinks, &tx),
                    ListResult::End => {
                        introspect2.get_server_info({
                            let sinks = sinks.clone();
                            let default_sink = default_sink.clone();
                            let tx = tx.clone();

                            move |info| set_default_sink(info, &sinks, &default_sink, &tx)
                        });
                    }
                    ListResult::Error => error!("Error while receiving sinks"),
                }
            });

            introspect.get_sink_input_info_list({
                let inputs = data.sink_inputs.clone();
                let tx = tx.clone();

                move |info| sink_input::add(info, &inputs, &tx)
            });

            let subscribe_callback = Box::new({
                let context = context.clone();
                let data = data.clone();
                let tx = tx.clone();

                move |facility, op, i| on_event(&context, &data, &tx, facility, op, i)
            });

            lock!(context).set_subscribe_callback(Some(subscribe_callback));
            lock!(context).subscribe(
                InterestMaskSet::SERVER | InterestMaskSet::SINK_INPUT | InterestMaskSet::SINK,
                |_| (),
            );
        }
        State::Failed => error!("Failed to connect to audio server"),
        State::Terminated => error!("Connection to audio server terminated"),
        _ => {}
    }
}

fn on_event(
    context: &Arc<Mutex<Context>>,
    data: &Data,
    tx: &broadcast::Sender<Event>,
    facility: Option<Facility>,
    op: Option<Operation>,
    i: u32,
) {
    let (Some(facility), Some(op)) = (facility, op) else {
        return;
    };

    match facility {
        Facility::Server => on_server_event(context, &data.sinks, &data.default_sink_name, tx),
        Facility::Sink => sink::on_event(context, &data.sinks, &data.default_sink_name, tx, op, i),
        Facility::SinkInput => sink_input::on_event(context, &data.sink_inputs, tx, op, i),
        _ => error!("Received unhandled facility: {facility:?}"),
    }
}

fn on_server_event(
    context: &Arc<Mutex<Context>>,
    sinks: &ArcMutVec<Sink>,
    default_sink: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
) {
    lock!(context).introspect().get_server_info({
        let sinks = sinks.clone();
        let default_sink = default_sink.clone();
        let tx = tx.clone();

        move |info| set_default_sink(info, &sinks, &default_sink, &tx)
    });
}

fn set_default_sink(
    info: &ServerInfo,
    sinks: &ArcMutVec<Sink>,
    default_sink: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
) {
    let default_sink_name = info.default_sink_name.as_ref().map(ToString::to_string);

    if default_sink_name != *lock!(default_sink) {
        if let Some(ref default_sink_name) = default_sink_name {
            if let Some(sink) = lock!(sinks)
                .iter_mut()
                .find(|s| s.name.as_str() == default_sink_name.as_str())
            {
                sink.active = true;
                debug!("Set sink active: {}", sink.name);
                tx.send_expect(Event::UpdateSink(sink.clone()));
            } else {
                warn!("Couldn't find sink: {}", default_sink_name);
            }
        }
    }

    *lock!(default_sink) = default_sink_name;
}

/// Converts a Pulse `ChannelVolumes` struct into a single percentage value,
/// representing the average value across all channels.
fn volume_to_percent(volume: ChannelVolumes) -> f64 {
    let avg = volume.avg().0;
    let base_delta = (Volume::NORMAL.0 - Volume::MUTED.0) as f64 / 100.0;

    ((avg - Volume::MUTED.0) as f64 / base_delta).round()
}

/// Converts a percentage volume into a Pulse volume value,
/// which can be used for setting channel volumes.
pub fn percent_to_volume(target_percent: f64) -> u32 {
    let base_delta = (Volume::NORMAL.0 as f32 - Volume::MUTED.0 as f32) / 100.0;

    if target_percent < 0.0 {
        Volume::MUTED.0
    } else if target_percent == 100.0 {
        Volume::NORMAL.0
    } else if target_percent >= 150.0 {
        (Volume::NORMAL.0 as f32 * 1.5) as u32
    } else if target_percent < 100.0 {
        Volume::MUTED.0 + target_percent as u32 * base_delta as u32
    } else {
        Volume::NORMAL.0 + (target_percent - 100.0) as u32 * base_delta as u32
    }
}

register_client!(Client, volume);
