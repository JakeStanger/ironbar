use super::handle::{ToplevelHandleData, ToplevelHandleDataExt, ToplevelHandleHandler};
use smithay_client_toolkit::error::GlobalError;
use smithay_client_toolkit::globals::{GlobalData, ProvidesBoundGlobal};
use std::marker::PhantomData;
use tracing::{debug, warn};
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::{event_created_child, Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    zwlr_foreign_toplevel_manager_v1::{Event, ZwlrForeignToplevelManagerV1},
};

pub struct ToplevelManagerState<V = ToplevelHandleData> {
    manager: ZwlrForeignToplevelManagerV1,
    _phantom: PhantomData<V>,
}

impl ToplevelManagerState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<ZwlrForeignToplevelManagerV1, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=3, GlobalData)?;
        debug!("Bound to ZwlForeignToplevelManagerV1 global");
        Ok(Self {
            manager,
            _phantom: PhantomData,
        })
    }
}

pub trait ToplevelManagerHandler: Sized {
    /// Advertises a new toplevel.
    fn toplevel(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        manager: ToplevelManagerState,
    );
}

impl ProvidesBoundGlobal<ZwlrForeignToplevelManagerV1, 3> for ToplevelManagerState {
    fn bound_global(&self) -> Result<ZwlrForeignToplevelManagerV1, GlobalError> {
        Ok(self.manager.clone())
    }
}

impl<D, V> Dispatch<ZwlrForeignToplevelManagerV1, GlobalData, D> for ToplevelManagerState<V>
where
    D: Dispatch<ZwlrForeignToplevelManagerV1, GlobalData>
        + Dispatch<ZwlrForeignToplevelHandleV1, V>
        + ToplevelManagerHandler
        + ToplevelHandleHandler
        + 'static,
    V: ToplevelHandleDataExt + Default + 'static + Send + Sync,
{
    event_created_child!(D, ZwlrForeignToplevelManagerV1, [
        0 => (ZwlrForeignToplevelHandleV1, V::default())
    ]);

    fn event(
        state: &mut D,
        toplevel_manager: &ZwlrForeignToplevelManagerV1,
        event: Event,
        _data: &GlobalData,
        conn: &Connection,
        qhandle: &QueueHandle<D>,
    ) {
        match event {
            Event::Toplevel { toplevel: _ } => {
                state.toplevel(
                    conn,
                    qhandle,
                    ToplevelManagerState {
                        manager: toplevel_manager.clone(),
                        _phantom: PhantomData,
                    },
                );
            }
            Event::Finished => {
                warn!("Foreign toplevel manager is no longer valid, but has not been dropped by client. This could cause window tracking issues.");
            }
            _ => {}
        }
    }
}
