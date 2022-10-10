use super::{Env, ToplevelHandler};
use crate::collection::Collection;
use crate::wayland::toplevel::{ToplevelEvent, ToplevelInfo};
use crate::wayland::toplevel_manager::listen_for_toplevels;
use crate::wayland::ToplevelChange;
use smithay_client_toolkit::environment::Environment;
use smithay_client_toolkit::output::{with_output_info, OutputInfo};
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::{new_default_environment, WaylandSource};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{broadcast, oneshot};
use tokio::task::spawn_blocking;
use tracing::trace;
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
};
use wayland_client::protocol::wl_seat::WlSeat;

pub struct WaylandClient {
    pub outputs: Vec<OutputInfo>,
    pub seats: Vec<WlSeat>,
    pub toplevels: Arc<RwLock<Collection<usize, (ToplevelInfo, ZwlrForeignToplevelHandleV1)>>>,
    toplevel_tx: broadcast::Sender<ToplevelEvent>,
    _toplevel_rx: broadcast::Receiver<ToplevelEvent>,
}

impl WaylandClient {
    pub(super) async fn new() -> Self {
        let (output_tx, output_rx) = oneshot::channel();
        let (seat_tx, seat_rx) = oneshot::channel();
        let (toplevel_tx, toplevel_rx) = broadcast::channel(32);

        let toplevel_tx2 = toplevel_tx.clone();

        let toplevels = Arc::new(RwLock::new(Collection::new()));
        let toplevels2 = toplevels.clone();

        // `queue` is not send so we need to handle everything inside the task
        spawn_blocking(move || {
            let (env, _display, queue) =
                new_default_environment!(Env, fields = [toplevel: ToplevelHandler::init()])
                    .expect("Failed to connect to Wayland compositor");

            let outputs = Self::get_outputs(&env);
            output_tx
                .send(outputs)
                .expect("Failed to send outputs out of task");

            let seats = env.get_all_seats();
            seat_tx.send(seats.into_iter().map(|seat| seat.detach()).collect::<Vec<WlSeat>>()).expect("Failed to send seats out of task");

            let _toplevel_manager = env.require_global::<ZwlrForeignToplevelManagerV1>();

            let _listener = listen_for_toplevels(env, move |handle, event, _ddata| {
                trace!("Received toplevel event: {:?}", event);

                if event.change != ToplevelChange::Close {
                    toplevels2
                        .write()
                        .expect("Failed to get write lock on toplevels")
                        .insert(event.toplevel.id, (event.toplevel.clone(), handle));
                } else {
                    toplevels2
                        .write()
                        .expect("Failed to get write lock on toplevels")
                        .remove(&event.toplevel.id);
                }

                toplevel_tx2
                    .send(event)
                    .expect("Failed to send toplevel event");
            });

            let mut event_loop = calloop::EventLoop::<()>::try_new().unwrap();
            WaylandSource::new(queue)
                .quick_insert(event_loop.handle())
                .unwrap();

            loop {
                // TODO: Avoid need for duration here - can we force some event when sending requests?
                event_loop.dispatch(Duration::from_millis(50), &mut ()).unwrap();
                event_loop.
            }
        });

        let outputs = output_rx
            .await
            .expect("Failed to receive outputs from task");

        let seats = seat_rx.await.expect("Failed to receive seats from task");

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
            .filter_map(|output| with_output_info(output, |info| info.clone()))
            .collect()
    }
}
