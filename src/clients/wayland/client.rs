use super::toplevel::{ToplevelEvent, ToplevelInfo};
use super::toplevel_manager::listen_for_toplevels;
use super::ToplevelChange;
use super::{Env, ToplevelHandler};
use crate::{error as err, send, write_lock};
use color_eyre::Report;
use indexmap::IndexMap;
use smithay_client_toolkit::environment::Environment;
use smithay_client_toolkit::output::{with_output_info, OutputInfo};
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::{new_default_environment, WaylandSource};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{broadcast, oneshot};
use tokio::task::spawn_blocking;
use tracing::{error, trace};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
};

pub struct WaylandClient {
    pub outputs: Vec<OutputInfo>,
    pub seats: Vec<WlSeat>,
    pub toplevels: Arc<RwLock<IndexMap<usize, (ToplevelInfo, ZwlrForeignToplevelHandleV1)>>>,
    toplevel_tx: broadcast::Sender<ToplevelEvent>,
    _toplevel_rx: broadcast::Receiver<ToplevelEvent>,
}

impl WaylandClient {
    pub(super) async fn new() -> Self {
        let (output_tx, output_rx) = oneshot::channel();
        let (seat_tx, seat_rx) = oneshot::channel();

        let (toplevel_tx, toplevel_rx) = broadcast::channel(32);

        let toplevel_tx2 = toplevel_tx.clone();

        let toplevels = Arc::new(RwLock::new(IndexMap::new()));
        let toplevels2 = toplevels.clone();

        // `queue` is not send so we need to handle everything inside the task
        spawn_blocking(move || {
            let (env, _display, queue) =
                new_default_environment!(Env, fields = [toplevel: ToplevelHandler::init()])
                    .expect("Failed to connect to Wayland compositor");

            let outputs = Self::get_outputs(&env);
            send!(output_tx, outputs);

            let seats = env.get_all_seats();
            send!(
                seat_tx,
                seats
                    .into_iter()
                    .map(|seat| seat.detach())
                    .collect::<Vec<WlSeat>>()
            );

            let _toplevel_manager = env.require_global::<ZwlrForeignToplevelManagerV1>();

            let _listener = listen_for_toplevels(env, move |handle, event, _ddata| {
                trace!("Received toplevel event: {:?}", event);

                if event.change == ToplevelChange::Close {
                    write_lock!(toplevels2).remove(&event.toplevel.id);
                } else {
                    write_lock!(toplevels2)
                        .insert(event.toplevel.id, (event.toplevel.clone(), handle));
                }

                send!(toplevel_tx2, event);
            });

            let mut event_loop =
                calloop::EventLoop::<()>::try_new().expect("Failed to create new event loop");
            WaylandSource::new(queue)
                .quick_insert(event_loop.handle())
                .expect("Failed to insert event loop into wayland event queue");

            loop {
                // TODO: Avoid need for duration here - can we force some event when sending requests?
                if let Err(err) = event_loop.dispatch(Duration::from_millis(50), &mut ()) {
                    error!(
                        "{:?}",
                        Report::new(err).wrap_err("Failed to dispatch pending wayland events")
                    );
                }
            }
        });

        let outputs = output_rx.await.expect(err::ERR_CHANNEL_RECV);

        let seats = seat_rx.await.expect(err::ERR_CHANNEL_RECV);

        Self {
            outputs,
            seats,
            toplevels,
            toplevel_tx,
            _toplevel_rx: toplevel_rx,
        }
    }

    pub fn subscribe_toplevels(&self) -> broadcast::Receiver<ToplevelEvent> {
        self.toplevel_tx.subscribe()
    }

    fn get_outputs(env: &Environment<Env>) -> Vec<OutputInfo> {
        let outputs = env.get_all_outputs();

        outputs
            .iter()
            .filter_map(|output| with_output_info(output, Clone::clone))
            .collect()
    }
}
