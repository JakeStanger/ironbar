use crate::lock;
use chrono::{DateTime, Utc};
use color_eyre::{Result, eyre::eyre};
use std::sync::Mutex;
use std::time::Duration;
use tracing::{debug, warn};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1;
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1;

struct Inhibitor {
    inner: ZwpIdleInhibitorV1,
    expiry: Option<DateTime<Utc>>,
}

impl Drop for Inhibitor {
    fn drop(&mut self) {
        self.inner.destroy();
    }
}

struct Noop;
wayland_client::delegate_noop!(Noop: ignore ZwpIdleInhibitorV1);
wayland_client::delegate_noop!(Noop: ignore ZwpIdleInhibitManagerV1);

struct RegistryState {
    manager: Option<ZwpIdleInhibitManagerV1>,
}

impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, ()> for RegistryState {
    fn event(
        state: &mut Self,
        registry: &wayland_client::protocol::wl_registry::WlRegistry,
        event: wayland_client::protocol::wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_registry::Event::Global {
            name,
            interface,
            version,
            ..
        } = event
            && interface == "zwp_idle_inhibit_manager_v1"
        {
            state.manager = Some(registry.bind(name, version.min(1), qh, ()));
        }
    }
}
wayland_client::delegate_noop!(RegistryState: ignore ZwpIdleInhibitManagerV1);

struct InhibitState {
    manager: ZwpIdleInhibitManagerV1,
    surface: WlSurface,
    queue: EventQueue<Noop>,
    active: Option<Inhibitor>,
}

impl std::fmt::Debug for InhibitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InhibitState")
            .field("active", &self.active.as_ref().and_then(|i| i.expiry))
            .finish_non_exhaustive()
    }
}

/// Manager for idle inhibit protocol on GTK's Wayland connection.
#[derive(Debug)]
pub struct IdleInhibitManager {
    state: Mutex<Option<InhibitState>>,
}

impl IdleInhibitManager {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    fn initialize(&self) -> Result<InhibitState> {
        use gdk4_wayland::prelude::WaylandSurfaceExtManual;
        use gtk::prelude::*;

        let surface = gtk::gio::Application::default()
            .and_then(|app| app.downcast::<gtk::Application>().ok())
            .and_then(|app| app.windows().first().cloned())
            .and_then(|w| w.surface())
            .and_then(|s| s.downcast::<gdk4_wayland::WaylandSurface>().ok())
            .and_then(|ws| ws.wl_surface())
            .ok_or_else(|| {
                eyre!("Failed to get Wayland surface (not running on Wayland or no GTK window)")
            })?;

        let conn = gtk::gdk::Display::default()
            .and_downcast::<gdk4_wayland::WaylandDisplay>()
            .and_then(|d| d.wl_display())
            .and_then(|d| d.backend().upgrade().map(Connection::from_backend))
            .ok_or_else(|| eyre!("Failed to get Wayland connection from GTK display"))?;

        let mut bind_queue = conn.new_event_queue();
        let mut registry_state = RegistryState { manager: None };

        conn.display().get_registry(&bind_queue.handle(), ());
        bind_queue
            .roundtrip(&mut registry_state)
            .map_err(|e| eyre!("Wayland registry roundtrip failed: {e}"))?;

        debug!("Bound idle inhibit manager");

        Ok(InhibitState {
            manager: registry_state
                .manager
                .ok_or_else(|| eyre!("Compositor does not support idle-inhibit protocol"))?,
            surface,
            queue: conn.new_event_queue(),
            active: None,
        })
    }

    fn with_state<F>(&self, f: F)
    where
        F: FnOnce(&mut InhibitState),
    {
        let mut state = lock!(self.state);
        if state.is_none() {
            match self.initialize() {
                Ok(st) => *state = Some(st),
                Err(e) => {
                    warn!("Idle inhibit unavailable: {e:?}");
                    return;
                }
            }
        }
        state.as_mut().map(f);
    }

    pub fn inhibit_start(&self, duration: Duration) {
        self.with_state(|st| {
            let expiry = if duration == Duration::MAX {
                None
            } else {
                chrono::Duration::from_std(duration)
                    .ok()
                    .and_then(|d| Utc::now().checked_add_signed(d))
            };

            st.active = Some(Inhibitor {
                inner: st
                    .manager
                    .create_inhibitor(&st.surface, &st.queue.handle(), ()),
                expiry,
            });
        });
    }

    pub fn inhibit_stop(&self) {
        self.with_state(|st| {
            st.active = None; // Drop auto-destroys
        });
    }

    pub fn inhibit_expiry(&self) -> Option<DateTime<Utc>> {
        lock!(self.state).as_ref()?.active.as_ref()?.expiry
    }
}
