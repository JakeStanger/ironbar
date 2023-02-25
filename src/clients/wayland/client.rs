use super::wlr_foreign_toplevel::{
    handle::{ToplevelEvent, ToplevelInfo},
    manager::listen_for_toplevels,
};
use super::{DData, Env, ToplevelHandler};
use crate::{error as err, send};
use cfg_if::cfg_if;
use color_eyre::Report;
use indexmap::IndexMap;
use smithay_client_toolkit::environment::Environment;
use smithay_client_toolkit::output::{with_output_info, OutputInfo};
use smithay_client_toolkit::reexports::calloop::channel::{channel, Event, Sender};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::WaylandSource;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, oneshot};
use tokio::task::spawn_blocking;
use tracing::{debug, error};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{ConnectError, Display, EventQueue};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
};

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        use super::{ClipboardItem};
        use super::wlr_data_control::manager::{listen_to_devices, DataControlDeviceHandler};
        use crate::{read_lock, write_lock};
        use tokio::spawn;
    }
}

#[derive(Debug)]
pub enum Request {
    /// Copies the value to the clipboard
    #[cfg(feature = "clipboard")]
    CopyToClipboard(Arc<ClipboardItem>),
    /// Forces a dispatch, flushing any currently queued events
    Refresh,
}

pub struct WaylandClient {
    pub outputs: Vec<OutputInfo>,
    pub seats: Vec<WlSeat>,

    pub toplevels: Arc<RwLock<IndexMap<usize, (ToplevelInfo, ZwlrForeignToplevelHandleV1)>>>,
    toplevel_tx: broadcast::Sender<ToplevelEvent>,
    _toplevel_rx: broadcast::Receiver<ToplevelEvent>,

    #[cfg(feature = "clipboard")]
    clipboard_tx: broadcast::Sender<Arc<ClipboardItem>>,
    #[cfg(feature = "clipboard")]
    clipboard: Arc<RwLock<Option<Arc<ClipboardItem>>>>,

    request_tx: Sender<Request>,
}

impl WaylandClient {
    pub(super) async fn new() -> Self {
        let (output_tx, output_rx) = oneshot::channel();
        let (seat_tx, seat_rx) = oneshot::channel();

        let (toplevel_tx, toplevel_rx) = broadcast::channel(32);

        let toplevels = Arc::new(RwLock::new(IndexMap::new()));
        let toplevels2 = toplevels.clone();

        let toplevel_tx2 = toplevel_tx.clone();

        cfg_if! {
            if #[cfg(feature = "clipboard")] {
                let (clipboard_tx, mut clipboard_rx) = broadcast::channel(32);
                let clipboard = Arc::new(RwLock::new(None));
                let clipboard_tx2 = clipboard_tx.clone();
            }
        }

        let (ev_tx, ev_rx) = channel::<Request>();

        // `queue` is not `Send` so we need to handle everything inside the task
        spawn_blocking(move || {
            let toplevels = toplevels2;
            let toplevel_tx = toplevel_tx2;

            let (env, _display, queue) =
                Self::new_environment().expect("Failed to connect to Wayland compositor");

            let mut event_loop =
                EventLoop::<DData>::try_new().expect("Failed to create new event loop");
            WaylandSource::new(queue)
                .quick_insert(event_loop.handle())
                .expect("Failed to insert Wayland event queue into event loop");

            let outputs = Self::get_outputs(&env);
            send!(output_tx, outputs);

            let seats = env.get_all_seats();

            // TODO: Actually handle seats properly
            #[cfg(feature = "clipboard")]
            let default_seat = seats[0].detach();

            send!(
                seat_tx,
                seats
                    .into_iter()
                    .map(|seat| seat.detach())
                    .collect::<Vec<WlSeat>>()
            );

            let handle = event_loop.handle();
            handle
                .insert_source(ev_rx, move |event, _metadata, ddata| {
                    // let env = &ddata.env;
                    match event {
                        Event::Msg(Request::Refresh) => debug!("Received refresh event"),
                        #[cfg(feature = "clipboard")]
                        Event::Msg(Request::CopyToClipboard(value)) => {
                            super::wlr_data_control::copy_to_clipboard(
                                &ddata.env,
                                &default_seat,
                                &value,
                            )
                            .expect("Failed to copy to clipboard");
                        }
                        Event::Closed => panic!("Channel unexpectedly closed"),
                    }
                })
                .expect("Failed to insert channel into event queue");

            let _toplevel_manager = env.require_global::<ZwlrForeignToplevelManagerV1>();

            let _toplevel_listener = listen_for_toplevels(&env, move |handle, event, _ddata| {
                super::wlr_foreign_toplevel::update_toplevels(
                    &toplevels,
                    handle,
                    event,
                    &toplevel_tx,
                );
            });

            cfg_if! {
                if #[cfg(feature = "clipboard")] {
                    let clipboard_tx = clipboard_tx2;
                    let handle = event_loop.handle();

                    let _offer_listener = listen_to_devices(&env, move |_seat, event, ddata| {
                        debug!("Received clipboard event");
                        super::wlr_data_control::receive_offer(event, &handle, clipboard_tx.clone(), ddata);
                    });
                }
            }

            let mut data = DData {
                env,
                offer_tokens: HashMap::new(),
            };

            loop {
                if let Err(err) = event_loop.dispatch(None, &mut data) {
                    error!(
                        "{:?}",
                        Report::new(err).wrap_err("Failed to dispatch pending wayland events")
                    );
                }
            }
        });

        // keep track of current clipboard item
        #[cfg(feature = "clipboard")]
        {
            let clipboard = clipboard.clone();
            spawn(async move {
                while let Ok(item) = clipboard_rx.recv().await {
                    let mut clipboard = write_lock!(clipboard);
                    clipboard.replace(item);
                }
            });
        }

        let outputs = output_rx.await.expect(err::ERR_CHANNEL_RECV);

        let seats = seat_rx.await.expect(err::ERR_CHANNEL_RECV);

        Self {
            outputs,
            seats,
            #[cfg(feature = "clipboard")]
            clipboard,
            toplevels,
            toplevel_tx,
            _toplevel_rx: toplevel_rx,
            #[cfg(feature = "clipboard")]
            clipboard_tx,
            request_tx: ev_tx,
        }
    }

    pub fn subscribe_toplevels(&self) -> broadcast::Receiver<ToplevelEvent> {
        self.toplevel_tx.subscribe()
    }

    #[cfg(feature = "clipboard")]
    pub fn subscribe_clipboard(&self) -> broadcast::Receiver<Arc<ClipboardItem>> {
        self.clipboard_tx.subscribe()
    }

    pub fn roundtrip(&self) {
        send!(self.request_tx, Request::Refresh);
    }

    #[cfg(feature = "clipboard")]
    pub fn get_clipboard(&self) -> Option<Arc<ClipboardItem>> {
        let clipboard = read_lock!(self.clipboard);
        clipboard.as_ref().cloned()
    }

    #[cfg(feature = "clipboard")]
    pub fn copy_to_clipboard(&self, item: Arc<ClipboardItem>) {
        send!(self.request_tx, Request::CopyToClipboard(item));
    }

    fn get_outputs(env: &Environment<Env>) -> Vec<OutputInfo> {
        let outputs = env.get_all_outputs();

        outputs
            .iter()
            .filter_map(|output| with_output_info(output, Clone::clone))
            .collect()
    }

    fn new_environment() -> Result<(Environment<Env>, Display, EventQueue), ConnectError> {
        Display::connect_to_env().and_then(|display| {
            let mut queue = display.create_event_queue();
            let ret = {
                let mut sctk_seats = smithay_client_toolkit::seat::SeatHandler::new();
                let sctk_data_device_manager =
                    smithay_client_toolkit::data_device::DataDeviceHandler::init(&mut sctk_seats);

                #[cfg(feature = "clipboard")]
                let data_control_device = DataControlDeviceHandler::init(&mut sctk_seats);

                let sctk_primary_selection_manager =
                    smithay_client_toolkit::primary_selection::PrimarySelectionHandler::init(
                        &mut sctk_seats,
                    );

                let display = ::smithay_client_toolkit::reexports::client::Proxy::clone(&display);
                let env = Environment::new(
                    &display.attach(queue.token()),
                    &mut queue,
                    Env {
                        sctk_compositor: smithay_client_toolkit::environment::SimpleGlobal::new(),
                        sctk_subcompositor: smithay_client_toolkit::environment::SimpleGlobal::new(
                        ),
                        sctk_shm: smithay_client_toolkit::shm::ShmHandler::new(),
                        sctk_outputs: smithay_client_toolkit::output::OutputHandler::new(),
                        sctk_seats,
                        sctk_data_device_manager,
                        sctk_primary_selection_manager,
                        toplevel: ToplevelHandler::init(),
                        #[cfg(feature = "clipboard")]
                        data_control_device,
                    },
                );

                if let Ok(env) = env.as_ref() {
                    let _psm = env.get_primary_selection_manager();
                }

                env
            };
            match ret {
                Ok(env) => Ok((env, display, queue)),
                Err(_e) => display.protocol_error().map_or_else(
                    || Err(ConnectError::NoCompositorListening),
                    |perr| {
                        panic!("[SCTK] A protocol error occured during initial setup: {perr}");
                    },
                ),
            }
        })
    }
}
