use smithay_client_toolkit::error::GlobalError;
use smithay_client_toolkit::globals::{GlobalData, ProvidesBoundGlobal};
use std::marker::PhantomData;
use tracing::{debug, warn};
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_hyprland::toplevel_export::v1::client::hyprland_toplevel_export_manager_v1::{HyprlandToplevelExportManagerV1, Event};
use wayland_protocols_hyprland::toplevel_export::v1::client::hyprland_toplevel_export_frame_v1::{
    HyprlandToplevelExportFrameV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    ZwlrForeignToplevelHandleV1,
};
use crate::clients::wayland::ToplevelHandle;
use super::frame::{ToplevelFrameData, ToplevelFrameDataExt, ToplevelFrameHandler};

#[derive(Debug)]
pub struct ToplevelManagerState<V = ToplevelFrameData> {
    manager: HyprlandToplevelExportManagerV1,
    _phantom: PhantomData<V>,
}

impl ToplevelManagerState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<HyprlandToplevelExportManagerV1, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=2, GlobalData)?;
        debug!(
            "Bound to HyprlandToplevelExportManagerV1 (v{}) global",
            manager.version()
        );
        Ok(Self {
            manager,
            _phantom: PhantomData,
        })
    }

    pub(super) fn capture<D, U>(
        &self,
        handle: &ZwlrForeignToplevelHandleV1,
        qh: &QueueHandle<D>,
        data: U,
    ) where
        D: Dispatch<HyprlandToplevelExportFrameV1, U> + 'static,
        U: ToplevelFrameDataExt + 'static,
    {
        self.manager
            .capture_toplevel_with_wlr_toplevel_handle(1, handle, qh, data);
    }
}

pub trait ToplevelManagerHandler: Sized {
    fn capture(&self, handle: &ToplevelHandle);
}

impl ProvidesBoundGlobal<HyprlandToplevelExportManagerV1, 2> for ToplevelManagerState {
    fn bound_global(&self) -> Result<HyprlandToplevelExportManagerV1, GlobalError> {
        Ok(self.manager.clone())
    }
}

impl<D, V> Dispatch<HyprlandToplevelExportManagerV1, GlobalData, D> for ToplevelManagerState<V>
where
    D: Dispatch<HyprlandToplevelExportManagerV1, GlobalData, D>
        + Dispatch<HyprlandToplevelExportFrameV1, V>
        + ToplevelManagerHandler
        + ToplevelFrameHandler
        + 'static,
    V: ToplevelFrameDataExt + Default + 'static + Send + Sync,
{
    event_created_child!(D, HyprlandToplevelExportManagerV1, [
        0 => (HyprlandToplevelExportFrameV1, V::default())
    ]);

    fn event(
        _state: &mut D,
        _proxy: &HyprlandToplevelExportManagerV1,
        event: Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        warn!("received unhandled event {:?}", event);
    }
}
