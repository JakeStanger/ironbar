mod dbus_keyboard;
mod dbus_login1;
mod fs_brightness;
#[cfg(feature = "ipc")]
mod ipc;

use crate::clients::ClientResult;
use crate::clients::brightness::dbus_login1::Login1SessionProxy;
use crate::clients::brightness::fs_brightness::FsLogin1Session;
use crate::register_fallible_client;
use dbus_keyboard::KbdBacklightProxy;
use std::sync::Arc;

pub use fs_brightness::default_resource_name;

#[derive(Debug)]
pub struct Client {
    keyboard: Arc<KbdBacklightProxy<'static>>,
    screen_writer: Arc<Login1SessionProxy<'static>>,
    screen_reader: Arc<FsLogin1Session>,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let dbus = Box::pin(zbus::Connection::system()).await?;

        let keyboard = KbdBacklightProxy::new(&dbus).await?;
        let screen_writer = Login1SessionProxy::new(&dbus).await?;
        let screen_reader = FsLogin1Session::default();

        Ok(Arc::new(Self {
            keyboard: Arc::new(keyboard),
            screen_writer: Arc::new(screen_writer),
            screen_reader: Arc::new(screen_reader),
        }))
    }

    pub fn keyboard(&self) -> &KbdBacklightProxy<'static> {
        &self.keyboard
    }

    pub fn screen_writer(&self) -> &Login1SessionProxy<'static> {
        &self.screen_writer
    }

    pub fn screen_reader(&self) -> &FsLogin1Session {
        &self.screen_reader
    }
}

register_fallible_client!(Client, brightness);
