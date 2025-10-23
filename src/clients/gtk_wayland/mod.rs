mod idle_inhibit;

use crate::register_client;
use idle_inhibit::IdleInhibitManager;
use std::ops::Deref;

/// Client wrapping GTK's Wayland connection and providing access
/// to protocol managers that need to share GTK's connection.
#[derive(Debug)]
pub struct Client {
    idle_inhibit: IdleInhibitManager,
}

impl Client {
    pub fn new() -> Self {
        let idle_inhibit = IdleInhibitManager::new();

        Self { idle_inhibit }
    }
}

impl Deref for Client {
    type Target = IdleInhibitManager;

    fn deref(&self) -> &Self::Target {
        &self.idle_inhibit
    }
}

register_client!(Client, gtk_wayland);
