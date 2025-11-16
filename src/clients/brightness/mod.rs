mod dbus_keyboard;
mod dbus_login1;
mod fs_reader;
#[cfg(feature = "ipc")]
mod ironvar;

use crate::clients::ClientResult;
use crate::clients::brightness::dbus_login1::Login1SessionProxy;
use crate::register_fallible_client;
use dbus_keyboard::KbdBacklightProxy;
use std::sync::Arc;

pub use fs_reader::{brightness, default_resource_name, max_brightness};

#[derive(Debug)]
pub struct Client {
    keyboard: Arc<KbdBacklightProxy<'static>>,
    screen_writer: Arc<Login1SessionProxy<'static>>,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let dbus = Box::pin(zbus::Connection::system()).await?;

        let keyboard = KbdBacklightProxy::new(&dbus).await?;
        let screen_writer = Login1SessionProxy::new(&dbus).await?;

        Ok(Arc::new(Self {
            keyboard: Arc::new(keyboard),
            screen_writer: Arc::new(screen_writer),
        }))
    }

    pub fn keyboard(&self) -> &KbdBacklightProxy<'static> {
        &self.keyboard
    }

    pub fn screen_writer(&self) -> &Login1SessionProxy<'static> {
        &self.screen_writer
    }
}

register_fallible_client!(Client, brightness);
