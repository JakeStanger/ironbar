use crate::{lock, register_client};
use gtk::ApplicationInhibitFlags;
use gtk::prelude::*;
use std::sync::{Arc, Mutex, Weak};
use tracing::{error, trace};

fn get_app() -> gtk::Application {
    gtk::gio::Application::default()
        .and_downcast()
        .expect("GTK application not initialized")
}

/// Cookie holder that uninhibits on drop
pub struct InhibitCookie(pub u32);

impl Drop for InhibitCookie {
    fn drop(&mut self) {
        get_app().uninhibit(self.0);
    }
}

/// Client for managing GTK application inhibit state.
///
/// Uses Weak reference to delegate ownership to modules - when all modules
/// drop their `InhibitCookie`, the system uninhibit is called automatically.
#[derive(Debug)]
pub struct Client {
    cookie: Mutex<Option<Weak<InhibitCookie>>>,
}

impl Client {
    pub(crate) fn new() -> Self {
        trace!("Initializing inhibit client");
        Self {
            cookie: Mutex::new(None),
        }
    }

    /// Acquires an inhibit cookie.
    ///
    /// Uses global inhibition (no window-specific behavior).
    /// Reuses existing cookie when multiple modules request inhibit.
    /// Returns None if platform doesn't support inhibit.
    pub fn acquire(&self) -> Option<Arc<InhibitCookie>> {
        let mut cookie_opt = lock!(self.cookie);

        // Upgrade weak cookie ref if it exists else create a new one
        cookie_opt.as_ref().and_then(Weak::upgrade).or_else(|| {
            let cookie = get_app().inhibit(
                None::<&gtk::Window>,
                ApplicationInhibitFlags::IDLE,
                Some("Ironbar inhibit"),
            );

            if cookie == 0 {
                error!("GTK inhibit failed - platform may not support it");
                return None;
            }

            trace!("Created inhibit cookie: {}", cookie);
            let rc = Arc::new(InhibitCookie(cookie));
            *cookie_opt = Some(Arc::downgrade(&rc));
            Some(rc)
        })
    }
}

register_client!(Client, inhibit);
