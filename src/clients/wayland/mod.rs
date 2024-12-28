mod macros;
mod wl_output;
mod wl_seat;

use crate::error::{ExitCode, ERR_CHANNEL_RECV};
use crate::{arc_mut, lock, register_client, spawn, spawn_blocking};
use std::process::exit;
use std::sync::{Arc, Mutex};

use crate::channels::SyncSenderExt;
use calloop_channel::Event::Msg;
use cfg_if::cfg_if;
use color_eyre::Report;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::channel as calloop_channel;
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::{
    delegate_output, delegate_registry, delegate_seat, registry_handlers,
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, trace};
use wayland_client::globals::registry_queue_init;
use wayland_client::{Connection, QueueHandle};
pub use wl_output::{OutputEvent, OutputEventType};

cfg_if! {
    if #[cfg(any(feature = "focused", feature = "launcher"))] {
        mod wlr_foreign_toplevel;
        use crate::{delegate_foreign_toplevel_handle, delegate_foreign_toplevel_manager};
        use wlr_foreign_toplevel::manager::ToplevelManagerState;
        pub use wlr_foreign_toplevel::{ToplevelEvent, ToplevelHandle, ToplevelInfo};

    }
}

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        mod wlr_data_control;

        use crate::{delegate_data_control_device, delegate_data_control_device_manager, delegate_data_control_offer, delegate_data_control_source};
        use self::wlr_data_control::device::DataControlDevice;
        use self::wlr_data_control::manager::DataControlDeviceManagerState;
        use self::wlr_data_control::source::CopyPasteSource;
        use self::wlr_data_control::SelectionOfferItem;
        use wayland_client::protocol::wl_seat::WlSeat;

        pub use wlr_data_control::{ClipboardItem, ClipboardValue};

        #[derive(Debug)]
        pub struct DataControlDeviceEntry {
            seat: WlSeat,
            device: DataControlDevice,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Output(OutputEvent),
    #[cfg(any(feature = "focused", feature = "launcher"))]
    Toplevel(ToplevelEvent),
    #[cfg(feature = "clipboard")]
    Clipboard(ClipboardItem),
}

#[derive(Debug)]
pub enum Request {
    Roundtrip,

    #[cfg(feature = "ipc")]
    OutputInfoAll,

    #[cfg(any(feature = "focused", feature = "launcher"))]
    ToplevelInfoAll,
    #[cfg(feature = "launcher")]
    ToplevelFocus(usize),
    #[cfg(feature = "launcher")]
    ToplevelMinimize(usize),

    #[cfg(feature = "clipboard")]
    CopyToClipboard(ClipboardItem),
    #[cfg(feature = "clipboard")]
    ClipboardItem,
}

#[derive(Debug)]
pub enum Response {
    /// An empty success response
    Ok,

    #[cfg(feature = "ipc")]
    OutputInfoAll(Vec<smithay_client_toolkit::output::OutputInfo>),

    #[cfg(any(feature = "focused", feature = "launcher"))]
    ToplevelInfoAll(Vec<ToplevelInfo>),

    #[cfg(feature = "clipboard")]
    ClipboardItem(Option<ClipboardItem>),
}

#[derive(Debug)]
#[allow(dead_code)]
struct BroadcastChannel<T>(broadcast::Sender<T>, Arc<Mutex<broadcast::Receiver<T>>>);

impl<T> From<(broadcast::Sender<T>, broadcast::Receiver<T>)> for BroadcastChannel<T> {
    fn from(value: (broadcast::Sender<T>, broadcast::Receiver<T>)) -> Self {
        Self(value.0, arc_mut!(value.1))
    }
}

#[derive(Debug)]
pub struct Client {
    tx: calloop_channel::Sender<Request>,
    rx: Arc<Mutex<std::sync::mpsc::Receiver<Response>>>,

    output_channel: BroadcastChannel<OutputEvent>,
    #[cfg(any(feature = "focused", feature = "launcher"))]
    toplevel_channel: BroadcastChannel<ToplevelEvent>,
    #[cfg(feature = "clipboard")]
    clipboard_channel: BroadcastChannel<ClipboardItem>,
}

impl Client {
    pub(crate) fn new() -> Self {
        let (event_tx, mut event_rx) = mpsc::channel(32);

        let (request_tx, request_rx) = calloop_channel::channel();
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let output_channel = broadcast::channel(32);
        #[cfg(any(feature = "focused", feature = "launcher"))]
        let toplevel_channel = broadcast::channel(32);

        #[cfg(feature = "clipboard")]
        let clipboard_channel = broadcast::channel(32);

        spawn_blocking(move || {
            Environment::spawn(event_tx, request_rx, response_tx);
        });

        // listen to events
        {
            let output_tx = output_channel.0.clone();
            #[cfg(any(feature = "focused", feature = "launcher"))]
            let toplevel_tx = toplevel_channel.0.clone();

            #[cfg(feature = "clipboard")]
            let clipboard_tx = clipboard_channel.0.clone();

            spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        Event::Output(event) => output_tx.send_expect(event),
                        #[cfg(any(feature = "focused", feature = "launcher"))]
                        Event::Toplevel(event) => toplevel_tx.send_expect(event),
                        #[cfg(feature = "clipboard")]
                        Event::Clipboard(item) => clipboard_tx.send_expect(item),
                    };
                }
            });
        }

        Self {
            tx: request_tx,
            rx: arc_mut!(response_rx),

            output_channel: output_channel.into(),
            #[cfg(any(feature = "focused", feature = "launcher"))]
            toplevel_channel: toplevel_channel.into(),
            #[cfg(feature = "clipboard")]
            clipboard_channel: clipboard_channel.into(),
        }
    }

    /// Sends a request to the environment event loop,
    /// and returns the response.
    fn send_request(&self, request: Request) -> Response {
        self.tx.send_expect(request);
        lock!(self.rx).recv().expect(ERR_CHANNEL_RECV)
    }

    /// Sends a round-trip request to the client,
    /// forcing it to send/receive any events in the queue.
    pub(crate) fn roundtrip(&self) -> Response {
        self.send_request(Request::Roundtrip)
    }
}

#[derive(Debug)]
pub struct Environment {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,

    queue_handle: QueueHandle<Self>,
    loop_handle: LoopHandle<'static, Self>,

    event_tx: mpsc::Sender<Event>,
    response_tx: std::sync::mpsc::Sender<Response>,

    // local state
    #[cfg(any(feature = "focused", feature = "launcher"))]
    handles: Vec<ToplevelHandle>,

    // -- clipboard --
    #[cfg(feature = "clipboard")]
    data_control_device_manager_state: DataControlDeviceManagerState,

    #[cfg(feature = "clipboard")]
    data_control_devices: Vec<DataControlDeviceEntry>,
    #[cfg(feature = "clipboard")]
    copy_paste_sources: Vec<CopyPasteSource>,
    #[cfg(feature = "clipboard")]
    selection_offers: Vec<SelectionOfferItem>,

    // local state
    #[cfg(feature = "clipboard")]
    clipboard: Arc<Mutex<Option<ClipboardItem>>>,
}

delegate_registry!(Environment);

delegate_output!(Environment);
delegate_seat!(Environment);

cfg_if! {
    if #[cfg(any(feature = "focused", feature = "launcher"))] {
        delegate_foreign_toplevel_manager!(Environment);
        delegate_foreign_toplevel_handle!(Environment);
    }
}

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        delegate_data_control_device_manager!(Environment);
        delegate_data_control_device!(Environment);
        delegate_data_control_offer!(Environment);
        delegate_data_control_source!(Environment);
    }
}

impl Environment {
    pub fn spawn(
        event_tx: mpsc::Sender<Event>,
        request_rx: calloop_channel::Channel<Request>,
        response_tx: std::sync::mpsc::Sender<Response>,
    ) {
        let conn = Connection::connect_to_env().expect("Failed to connect to Wayland compositor");
        let (globals, queue) =
            registry_queue_init(&conn).expect("Failed to retrieve Wayland globals");

        let qh = queue.handle();
        let mut event_loop = EventLoop::<Self>::try_new().expect("Failed to create new event loop");

        WaylandSource::new(conn, queue)
            .insert(event_loop.handle())
            .expect("Failed to insert Wayland event queue into event loop");

        let loop_handle = event_loop.handle();

        // Initialize the registry handling
        // so other parts of Smithay's client toolkit may bind globals.
        let registry_state = RegistryState::new(&globals);

        let output_state = OutputState::new(&globals, &qh);
        let seat_state = SeatState::new(&globals, &qh);
        #[cfg(any(feature = "focused", feature = "launcher"))]
        ToplevelManagerState::bind(&globals, &qh)
            .expect("to bind to wlr_foreign_toplevel_manager global");

        #[cfg(feature = "clipboard")]
        let data_control_device_manager_state = DataControlDeviceManagerState::bind(&globals, &qh)
            .expect("to bind to wlr_data_control_device_manager global");

        let mut env = Self {
            registry_state,
            output_state,
            seat_state,
            #[cfg(feature = "clipboard")]
            data_control_device_manager_state,
            queue_handle: qh,
            loop_handle: loop_handle.clone(),
            event_tx,
            response_tx,
            #[cfg(any(feature = "focused", feature = "launcher"))]
            handles: vec![],

            #[cfg(feature = "clipboard")]
            data_control_devices: vec![],
            #[cfg(feature = "clipboard")]
            copy_paste_sources: vec![],
            #[cfg(feature = "clipboard")]
            selection_offers: vec![],
            #[cfg(feature = "clipboard")]
            clipboard: arc_mut!(None),
        };

        loop_handle
            .insert_source(request_rx, Self::on_request)
            .expect("to be able to insert source");

        loop {
            trace!("Dispatching event loop");
            if let Err(err) = event_loop.dispatch(None, &mut env) {
                error!(
                    "{:?}",
                    Report::new(err).wrap_err("Failed to dispatch pending wayland events")
                );

                exit(ExitCode::WaylandDispatchError as i32)
            }
        }
    }

    /// Processes a request from the client
    /// and sends the response.
    fn on_request(event: calloop_channel::Event<Request>, _metadata: &mut (), env: &mut Self) {
        trace!("Request: {event:?}");

        match event {
            Msg(Request::Roundtrip) => {
                debug!("received roundtrip request");
                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "ipc")]
            Msg(Request::OutputInfoAll) => {
                let infos = env.output_info_all();
                env.response_tx.send_expect(Response::OutputInfoAll(infos));
            }
            #[cfg(any(feature = "focused", feature = "launcher"))]
            Msg(Request::ToplevelInfoAll) => {
                let infos = env
                    .handles
                    .iter()
                    .filter_map(ToplevelHandle::info)
                    .collect();
                env.response_tx
                    .send_expect(Response::ToplevelInfoAll(infos));
            }
            #[cfg(feature = "launcher")]
            Msg(Request::ToplevelFocus(id)) => {
                let handle = env
                    .handles
                    .iter()
                    .find(|handle| handle.info().map_or(false, |info| info.id == id));

                if let Some(handle) = handle {
                    let seat = env.default_seat();
                    handle.focus(&seat);
                }

                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "launcher")]
            Msg(Request::ToplevelMinimize(id)) => {
                let handle = env
                    .handles
                    .iter()
                    .find(|handle| handle.info().map_or(false, |info| info.id == id));

                if let Some(handle) = handle {
                    handle.minimize();
                }

                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "clipboard")]
            Msg(Request::CopyToClipboard(item)) => {
                env.copy_to_clipboard(item);
                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "clipboard")]
            Msg(Request::ClipboardItem) => {
                let item = lock!(env.clipboard).clone();
                env.response_tx.send_expect(Response::ClipboardItem(item));
            }
            calloop_channel::Event::Closed => error!("request channel unexpectedly closed"),
        }
    }
}

impl ProvidesRegistryState for Environment {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

register_client!(Client, wayland);
