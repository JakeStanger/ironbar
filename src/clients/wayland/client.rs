use super::wlr_foreign_toplevel::handle::ToplevelHandle;
use super::wlr_foreign_toplevel::manager::ToplevelManagerState;
use super::wlr_foreign_toplevel::ToplevelEvent;
use super::Environment;
use crate::error::ERR_CHANNEL_RECV;
use crate::{send, spawn_blocking};
use cfg_if::cfg_if;
use color_eyre::Report;
use smithay_client_toolkit::output::{OutputInfo, OutputState};
use smithay_client_toolkit::reexports::calloop::channel::{channel, Event, Sender};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use std::collections::HashMap;
use std::sync::mpsc;
use tokio::sync::broadcast;
use tracing::{debug, error, trace};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::Connection;

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        use super::ClipboardItem;
        use super::wlr_data_control::manager::DataControlDeviceManagerState;
        use crate::lock;
        use std::sync::Arc;
    }
}

#[derive(Debug)]
pub enum Request {
    /// Sends a request for all the outputs.
    /// These are then sent on the `output` channel.
    Outputs,
    /// Sends a request for all the seats.
    /// These are then sent ont the `seat` channel.
    Seats,
    /// Sends a request for all the toplevels.
    /// These are then sent on the `toplevel_init` channel.
    Toplevels,
    /// Sends a request for the current clipboard item.
    /// This is then sent on the `clipboard_init` channel.
    #[cfg(feature = "clipboard")]
    Clipboard,
    /// Copies the value to the clipboard
    #[cfg(feature = "clipboard")]
    CopyToClipboard(Arc<ClipboardItem>),
    /// Forces a dispatch, flushing any currently queued events
    Roundtrip,
}

pub struct WaylandClient {
    // External channels
    toplevel_tx: broadcast::Sender<ToplevelEvent>,
    _toplevel_rx: broadcast::Receiver<ToplevelEvent>,
    #[cfg(feature = "clipboard")]
    clipboard_tx: broadcast::Sender<Arc<ClipboardItem>>,
    #[cfg(feature = "clipboard")]
    _clipboard_rx: broadcast::Receiver<Arc<ClipboardItem>>,

    // Internal channels
    toplevel_init_rx: mpsc::Receiver<HashMap<usize, ToplevelHandle>>,
    output_rx: mpsc::Receiver<Vec<OutputInfo>>,
    seat_rx: mpsc::Receiver<Vec<WlSeat>>,
    #[cfg(feature = "clipboard")]
    clipboard_init_rx: mpsc::Receiver<Option<Arc<ClipboardItem>>>,

    request_tx: Sender<Request>,
}

impl WaylandClient {
    pub(super) fn new() -> Self {
        let (toplevel_tx, toplevel_rx) = broadcast::channel(32);

        let (toplevel_init_tx, toplevel_init_rx) = mpsc::channel();
        #[cfg(feature = "clipboard")]
        let (clipboard_init_tx, clipboard_init_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();
        let (seat_tx, seat_rx) = mpsc::channel();

        let toplevel_tx2 = toplevel_tx.clone();

        cfg_if! {
            if #[cfg(feature = "clipboard")] {
                let (clipboard_tx, clipboard_rx) = broadcast::channel(32);
                let clipboard_tx2 = clipboard_tx.clone();
            }
        }

        let (ev_tx, ev_rx) = channel::<Request>();

        // `queue` is not `Send` so we need to handle everything inside the task
        spawn_blocking(move || {
            let toplevel_tx = toplevel_tx2;
            #[cfg(feature = "clipboard")]
            let clipboard_tx = clipboard_tx2;

            let conn =
                Connection::connect_to_env().expect("Failed to connect to Wayland compositor");
            let (globals, queue) =
                registry_queue_init(&conn).expect("Failed to retrieve Wayland globals");

            let qh = queue.handle();
            let mut event_loop =
                EventLoop::<Environment>::try_new().expect("Failed to create new event loop");

            WaylandSource::new(conn, queue)
                .insert(event_loop.handle())
                .expect("Failed to insert Wayland event queue into event loop");

            let loop_handle = event_loop.handle();

            // Initialize the registry handling
            // so other parts of Smithay's client toolkit may bind globals.
            let registry_state = RegistryState::new(&globals);

            let output_delegate = OutputState::new(&globals, &qh);
            let seat_delegate = SeatState::new(&globals, &qh);

            #[cfg(feature = "clipboard")]
            let data_control_device_manager_delegate =
                DataControlDeviceManagerState::bind(&globals, &qh)
                    .expect("data device manager is not available");

            let foreign_toplevel_manager_delegate = ToplevelManagerState::bind(&globals, &qh)
                .expect("foreign toplevel manager is not available");

            let mut env = Environment {
                registry_state,
                output_state: output_delegate,
                seat_state: seat_delegate,
                #[cfg(feature = "clipboard")]
                data_control_device_manager_state: data_control_device_manager_delegate,
                foreign_toplevel_manager_state: foreign_toplevel_manager_delegate,
                seats: vec![],
                handles: HashMap::new(),
                #[cfg(feature = "clipboard")]
                clipboard: crate::arc_mut!(None),
                toplevel_tx,
                #[cfg(feature = "clipboard")]
                clipboard_tx,
                #[cfg(feature = "clipboard")]
                data_control_devices: vec![],
                #[cfg(feature = "clipboard")]
                selection_offers: vec![],
                #[cfg(feature = "clipboard")]
                copy_paste_sources: vec![],
                loop_handle: event_loop.handle(),
            };

            loop_handle
                .insert_source(ev_rx, move |event, _metadata, env| {
                    trace!("{event:?}");
                    match event {
                        Event::Msg(Request::Roundtrip) => debug!("Received refresh event"),
                        Event::Msg(Request::Outputs) => {
                            trace!("Received get outputs request");

                            send!(output_tx, env.output_info());
                        }
                        Event::Msg(Request::Seats) => {
                            trace!("Receive get seats request");
                            send!(seat_tx, env.seats.clone());
                        }
                        Event::Msg(Request::Toplevels) => {
                            trace!("Receive get toplevels request");
                            send!(toplevel_init_tx, env.handles.clone());
                        }
                        #[cfg(feature = "clipboard")]
                        Event::Msg(Request::Clipboard) => {
                            trace!("Receive get clipboard requests");
                            let clipboard = lock!(env.clipboard).clone();
                            send!(clipboard_init_tx, clipboard);
                        }
                        #[cfg(feature = "clipboard")]
                        Event::Msg(Request::CopyToClipboard(value)) => {
                            env.copy_to_clipboard(value, &qh);
                        }
                        Event::Closed => panic!("Channel unexpectedly closed"),
                    }
                })
                .expect("Failed to insert channel into event queue");

            loop {
                trace!("Dispatching event loop");
                if let Err(err) = event_loop.dispatch(None, &mut env) {
                    error!(
                        "{:?}",
                        Report::new(err).wrap_err("Failed to dispatch pending wayland events")
                    );
                }
            }
        });

        Self {
            toplevel_tx,
            _toplevel_rx: toplevel_rx,
            toplevel_init_rx,
            #[cfg(feature = "clipboard")]
            clipboard_init_rx,
            output_rx,
            seat_rx,
            #[cfg(feature = "clipboard")]
            clipboard_tx,
            #[cfg(feature = "clipboard")]
            _clipboard_rx: clipboard_rx,
            request_tx: ev_tx,
        }
    }

    pub fn subscribe_toplevels(
        &self,
    ) -> (
        broadcast::Receiver<ToplevelEvent>,
        HashMap<usize, ToplevelHandle>,
    ) {
        let rx = self.toplevel_tx.subscribe();

        let receiver = &self.toplevel_init_rx;
        send!(self.request_tx, Request::Toplevels);
        let data = receiver.recv().expect(ERR_CHANNEL_RECV);

        (rx, data)
    }

    #[cfg(feature = "clipboard")]
    pub fn subscribe_clipboard(
        &self,
    ) -> (
        broadcast::Receiver<Arc<ClipboardItem>>,
        Option<Arc<ClipboardItem>>,
    ) {
        let rx = self.clipboard_tx.subscribe();

        let receiver = &self.clipboard_init_rx;
        send!(self.request_tx, Request::Clipboard);
        let data = receiver.recv().expect(ERR_CHANNEL_RECV);

        (rx, data)
    }

    /// Force a roundtrip on the wayland connection,
    /// flushing any queued events and immediately receiving any new ones.
    pub fn roundtrip(&self) {
        trace!("Sending roundtrip request");
        send!(self.request_tx, Request::Roundtrip);
    }

    pub fn get_outputs(&self) -> Vec<OutputInfo> {
        trace!("Sending get outputs request");

        send!(self.request_tx, Request::Outputs);
        self.output_rx.recv().expect(ERR_CHANNEL_RECV)
    }

    pub fn get_seats(&self) -> Vec<WlSeat> {
        trace!("Sending get seats request");

        send!(self.request_tx, Request::Seats);
        self.seat_rx.recv().expect(ERR_CHANNEL_RECV)
    }

    #[cfg(feature = "clipboard")]
    pub fn copy_to_clipboard(&self, item: Arc<ClipboardItem>) {
        send!(self.request_tx, Request::CopyToClipboard(item));
    }
}
