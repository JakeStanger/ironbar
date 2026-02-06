mod sink;
mod sink_input;
mod source;
mod source_output;

use crate::channels::SyncSenderExt;
use crate::{APP_ID, arc_mut, lock, register_client, spawn_blocking};
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::{Introspector, ServerInfo};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context, FlagSet, State};
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::proplist::Proplist;
use libpulse_binding::volume::{ChannelVolumes, Volume};
pub use sink::Sink;
pub use sink_input::SinkInput;
pub use source::Source;
pub use source_output::SourceOutput;

use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace, warn};

type ArcMutVec<T> = Arc<Mutex<Vec<T>>>;

trait HasIndex {
    fn index(&self) -> u32;
}

trait PulseObject<'a>: Sized + HasIndex {
    type Inner: 'a + Debug + HasIndex;

    fn name(&self) -> String;
    fn active(&self) -> bool;
    fn set_active(&mut self, active: bool);

    fn add_event(info: Self) -> Event;
    fn update_event(info: Self) -> Event;
    fn remove_event(info: Self) -> Event;

    fn add(
        result: ListResult<&'a Self::Inner>,
        items: &ArcMutVec<Self>,
        tx: &broadcast::Sender<Event>,
    ) where
        Self: From<&'a Self::Inner>,
    {
        let ListResult::Item(info) = result else {
            return;
        };

        trace!("adding {info:?}");
        lock!(items).push(info.into());
        tx.send_expect(Self::add_event(info.into()));
    }

    fn update(
        result: ListResult<&'a Self::Inner>,
        items: &ArcMutVec<Self>,
        default: Option<&Arc<Mutex<Option<String>>>>,
        tx: &broadcast::Sender<Event>,
    ) where
        Self: From<&'a Self::Inner>,
    {
        let ListResult::Item(info) = result else {
            return;
        };

        trace!("updating {info:?}");
        {
            let mut items = lock!(items);
            let Some(pos) = items.iter().position(|item| item.index() == info.index()) else {
                error!("received update to untracked item");
                return;
            };
            items[pos] = info.into();

            // update in local copy
            if let Some(default) = default.as_ref()
                && !items[pos].active()
                && let Some(default_item) = &*lock!(default)
            {
                let name = &items[pos].name();
                items[pos].set_active(name == default_item);
            }
        }

        // update in broadcast copy
        let mut item: Self = info.into();
        if let Some(default) = default.as_ref()
            && !item.active()
            && let Some(default_item) = &*lock!(default)
        {
            item.set_active(&item.name() == default_item);
        }

        tx.send_expect(Self::update_event(item));
    }

    fn remove(index: u32, items: &ArcMutVec<Self>, tx: &broadcast::Sender<Event>) {
        trace!("removing {index}");

        let mut sources = lock!(items);
        if let Some(pos) = sources.iter().position(|s| s.index() == index) {
            let info = sources.remove(pos);
            tx.send_expect(Self::remove_event(info));
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    AddSink(Sink),
    UpdateSink(Sink),
    RemoveSink(String),

    AddSource(Source),
    UpdateSource(Source),
    RemoveSource(String),

    AddInput(SinkInput),
    UpdateInput(SinkInput),
    RemoveInput(u32),

    AddOutput(SourceOutput),
    UpdateOutput(SourceOutput),
    RemoveOutput(u32),
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
    sources: ArcMutVec<Source>,
    source_outputs: ArcMutVec<SourceOutput>,

    default_sink_name: Arc<Mutex<Option<String>>>,
    default_source_name: Arc<Mutex<Option<String>>>,
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
            let introspect_sink = lock!(context).introspect();
            let introspect_source = lock!(context).introspect();

            introspect.get_sink_info_list({
                let sinks = data.sinks.clone();
                let default_sink = data.default_sink_name.clone();

                let tx = tx.clone();

                move |info| match info {
                    ListResult::Item(_) => Sink::add(info, &sinks, &tx),
                    ListResult::End => {
                        introspect_sink.get_server_info({
                            let sinks = sinks.clone();
                            let default_sink = default_sink.clone();
                            let tx = tx.clone();

                            move |info| set_default_sink(info, &sinks, &default_sink, &tx)
                        });
                    }
                    ListResult::Error => error!("Error while receiving sinks"),
                }
            });

            introspect.get_source_info_list({
                let sources = data.sources.clone();
                let default_source = data.default_source_name.clone();

                let tx = tx.clone();

                move |info| match info {
                    ListResult::Item(_) => Source::add(info, &sources, &tx),
                    ListResult::End => {
                        introspect_source.get_server_info({
                            let sources = sources.clone();
                            let default_source = default_source.clone();
                            let tx = tx.clone();

                            move |info| set_default_source(info, &sources, &default_source, &tx)
                        });
                    }
                    ListResult::Error => error!("Error while receiving sinks"),
                }
            });

            introspect.get_sink_input_info_list({
                let inputs = data.sink_inputs.clone();
                let tx = tx.clone();
                move |info| SinkInput::add(info, &inputs, &tx)
            });

            introspect.get_source_output_info_list({
                let outputs = data.source_outputs.clone();
                let tx = tx.clone();
                move |info| SourceOutput::add(info, &outputs, &tx)
            });

            let subscribe_callback = Box::new({
                let context = context.clone();
                let data = data.clone();
                let tx = tx.clone();

                move |facility, op, i| on_event(&context, &data, &tx, facility, op, i)
            });

            lock!(context).set_subscribe_callback(Some(subscribe_callback));
            lock!(context).subscribe(
                InterestMaskSet::SERVER
                    | InterestMaskSet::SINK_INPUT
                    | InterestMaskSet::SINK
                    | InterestMaskSet::SOURCE_OUTPUT
                    | InterestMaskSet::SOURCE,
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

    trace!("server event: {facility:?}, op: {op:?}, i: {i}");

    match facility {
        Facility::Server => on_server_event(
            context,
            &data.sinks,
            &data.sources,
            &data.default_sink_name,
            &data.default_source_name,
            tx,
        ),
        Facility::Sink => Sink::on_event(context, &data.sinks, &data.default_sink_name, tx, op, i),
        Facility::Source => {
            Source::on_event(context, &data.sources, &data.default_source_name, tx, op, i)
        }
        Facility::SinkInput => SinkInput::on_event(context, &data.sink_inputs, tx, op, i),
        Facility::SourceOutput => SourceOutput::on_event(context, &data.source_outputs, tx, op, i),
        _ => error!("Received unhandled facility: {facility:?}"),
    }
}

fn on_server_event(
    context: &Arc<Mutex<Context>>,
    sinks: &ArcMutVec<Sink>,
    sources: &ArcMutVec<Source>,
    default_sink: &Arc<Mutex<Option<String>>>,
    default_source: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
) {
    lock!(context).introspect().get_server_info({
        let sinks = sinks.clone();
        let default_sink = default_sink.clone();
        let sources = sources.clone();
        let default_source = default_source.clone();
        let tx = tx.clone();

        move |info| {
            set_default_sink(info, &sinks, &default_sink, &tx);
            set_default_source(info, &sources, &default_source, &tx);
        }
    });
}

fn set_default_sink(
    info: &ServerInfo,
    sinks: &ArcMutVec<Sink>,
    default_sink: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
) {
    let default_sink_name = info.default_sink_name.as_ref().map(ToString::to_string);

    if default_sink_name != *lock!(default_sink)
        && let Some(ref default_sink_name) = default_sink_name
    {
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

    *lock!(default_sink) = default_sink_name;
}

fn set_default_source(
    info: &ServerInfo,
    sources: &ArcMutVec<Source>,
    default_source: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
) {
    let default_source_name = info.default_source_name.as_ref().map(ToString::to_string);

    if default_source_name != *lock!(default_source)
        && let Some(ref default_source_name) = default_source_name
    {
        if let Some(source) = lock!(sources)
            .iter_mut()
            .find(|s| s.name.as_str() == default_source_name.as_str())
        {
            source.active = true;
            debug!("Set source active: {}", source.name);
            tx.send_expect(Event::UpdateSource(source.clone()));
        } else {
            warn!("Couldn't find source: {}", default_source_name);
        }
    }

    *lock!(default_source) = default_source_name;
}

#[derive(Debug, Clone)]
pub struct VolumeLevels(Vec<u32>);

impl VolumeLevels {
    pub fn percent(&self) -> f64 {
        let avg: u32 = self.iter().sum::<u32>() / self.len() as u32;
        let base_delta = (Volume::NORMAL.0 - Volume::MUTED.0) as f64 / 100.0;

        ((avg - Volume::MUTED.0) as f64 / base_delta).round()
    }

    pub fn set_percent(&mut self, percent: f64) {
        let volume = percent_to_volume(percent);
        self.fill(volume);
    }
}

impl Deref for VolumeLevels {
    type Target = Vec<u32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VolumeLevels {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<VolumeLevels> for ChannelVolumes {
    fn from(levels: VolumeLevels) -> Self {
        let mut cv = ChannelVolumes::default();
        cv.set_len(levels.len() as u8);
        cv.get_mut()
            .copy_from_slice(unsafe { std::mem::transmute::<&[u32], &[Volume]>(&levels) });
        cv
    }
}

impl From<ChannelVolumes> for VolumeLevels {
    fn from(value: ChannelVolumes) -> Self {
        let levels: &[u32] =
            unsafe { &*(std::ptr::from_ref::<[Volume]>(value.get()) as *const [u32]) };
        Self(Vec::from(&levels[..value.len() as usize]))
    }
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
