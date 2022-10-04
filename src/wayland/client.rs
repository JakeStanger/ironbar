use std::sync::{Arc, RwLock};
use super::{Env, ToplevelHandler};
use crate::wayland::toplevel_manager::listen_for_toplevels;
use smithay_client_toolkit::environment::Environment;
use smithay_client_toolkit::output::{with_output_info, OutputInfo};
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::{new_default_environment, WaylandSource};
use tokio::sync::{broadcast, oneshot};
use tokio::task::spawn_blocking;
use tracing::{trace};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;
use crate::collection::Collection;
use crate::wayland::toplevel::{ToplevelEvent, ToplevelInfo};
use crate::wayland::ToplevelChange;

pub struct WaylandClient {
    pub outputs: Vec<OutputInfo>,
    pub toplevels: Arc<RwLock<Collection<String, ToplevelInfo>>>,
    toplevel_tx: broadcast::Sender<ToplevelEvent>,
    _toplevel_rx: broadcast::Receiver<ToplevelEvent>,
}

impl WaylandClient {
    pub(super) async fn new() -> Self {
        let (output_tx, output_rx) = oneshot::channel();
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

            let _toplevel_manager = env.require_global::<ZwlrForeignToplevelManagerV1>();

            let _listener = listen_for_toplevels(env, move |_handle, event, _ddata| {
                trace!("Received toplevel event: {:?}", event);

                if event.change != ToplevelChange::Close {
                    toplevels2
                        .write()
                        .expect("Failed to get write lock on toplevels")
                        .insert(event.toplevel.app_id.clone(), event.toplevel.clone());
                } else {
                    toplevels2
                        .write()
                        .expect("Failed to get write lock on toplevels")
                        .remove(&event.toplevel.app_id);
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
                event_loop.dispatch(None, &mut ()).unwrap();
            }
        });

        let outputs = output_rx
            .await
            .expect("Failed to receive outputs from task");

        // spawn(async move {
        //     println!("start");
        //     while let Ok(ev) = toplevel_rx.recv().await {
        //         println!("recv {:?}", ev)
        //     }
        //     println!("stop");
        // });

        Self {
            outputs,
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
