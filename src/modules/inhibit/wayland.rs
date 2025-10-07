use chrono::{DateTime, Utc};
use color_eyre::Result;
use std::time::Duration;
use wayland_client::globals::GlobalListContents;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1;
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1;

pub struct WaylandBackend {
    conn: Connection,
    manager: ZwpIdleInhibitManagerV1,
    surface: wayland_client::protocol::wl_surface::WlSurface,
    qh: QueueHandle<WaylandAppData>,
    inhibitor: Option<ZwpIdleInhibitorV1>,
    pub(super) expiry: Option<DateTime<Utc>>,
}

struct WaylandAppData;

impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, GlobalListContents>
    for WaylandAppData
{
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_registry::WlRegistry,
        _: <wayland_client::protocol::wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(WaylandAppData: wayland_client::protocol::wl_compositor::WlCompositor);
delegate_noop!(WaylandAppData: wayland_client::protocol::wl_surface::WlSurface);
delegate_noop!(WaylandAppData: wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1);
delegate_noop!(WaylandAppData: wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1);

impl WaylandBackend {
    pub async fn new() -> Result<Self> {
        let conn = Connection::connect_to_env()?;
        let (globals, mut event_queue) = wayland_client::globals::registry_queue_init(&conn)?;
        let qh = event_queue.handle();
        let manager = globals.bind(&qh, 1..=1, ())?;
        let compositor: wayland_client::protocol::wl_compositor::WlCompositor =
            globals.bind(&qh, 1..=1, ())?;
        let surface = compositor.create_surface(&qh, ());
        event_queue.roundtrip(&mut WaylandAppData)?;
        Ok(Self {
            conn,
            manager,
            surface,
            qh,
            inhibitor: None,
            expiry: None,
        })
    }
}

impl Drop for WaylandBackend {
    fn drop(&mut self) {
        if let Some(i) = self.inhibitor.take() {
            i.destroy();
        }
        self.surface.destroy();
        let _ = self.conn.flush();
    }
}

impl WaylandBackend {
    pub async fn start(&mut self, duration: Duration) -> Result<()> {
        self.stop().await?;
        let inhibitor = self.manager.create_inhibitor(&self.surface, &self.qh, ());
        self.conn.flush()?;
        self.inhibitor = Some(inhibitor);
        self.expiry = super::calculate_expiry(duration);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(i) = self.inhibitor.take() {
            i.destroy();
            self.conn.flush()?;
        }
        self.expiry = None;
        Ok(())
    }
}
