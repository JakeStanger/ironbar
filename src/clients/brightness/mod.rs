mod dbus_keyboard;
mod dbus_login1;
mod fs_brightness;

use crate::await_sync;
use crate::clients::ClientResult;
use crate::clients::brightness::dbus_login1::Login1SessionProxy;
use crate::clients::brightness::fs_brightness::FsLogin1Session;
use crate::register_fallible_client;
use dbus_keyboard::KbdBacklightProxy;
use std::collections::HashMap;
use std::sync::Arc;

pub use fs_brightness::default_resource_name;

#[derive(Debug)]
pub struct Client {
    keyboard: KbdBacklightProxy<'static>,
    screen_writer: Login1SessionProxy<'static>,
    screen_reader: FsLogin1Session,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let dbus = Box::pin(zbus::Connection::system()).await?;

        let keyboard = KbdBacklightProxy::new(&dbus).await?;
        let screen_writer = Login1SessionProxy::new(&dbus).await?;
        let screen_reader = FsLogin1Session::default();

        Ok(Arc::new(Self {
            keyboard,
            screen_writer,
            screen_reader,
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

const KEY_KDB_MAX_BRIGHTNESS: &str = "KbdBacklight.GetMaxBrightness";
const KEY_KDB_BRIGHTNESS: &str = "KbdBacklight.GetBrightness";

#[cfg(feature = "ipc")]
impl crate::ironvar::Namespace for Client {
    fn get(&self, key: &str) -> Option<String> {
        match key {
            KEY_KDB_MAX_BRIGHTNESS => await_sync(self.keyboard.get_max_brightness())
                .ok()
                .map(|v| v.to_string()),

            KEY_KDB_BRIGHTNESS => await_sync(self.keyboard.get_max_brightness())
                .ok()
                .map(|v| v.to_string()),

            _ => None,
        }
    }

    fn list(&self) -> Vec<String> {
        self.get_all().keys().map(ToString::to_string).collect()
    }

    fn get_all(&self) -> HashMap<Box<str>, String> {
        let mut properties = HashMap::new();
        if let Ok(value) = await_sync(self.keyboard.get_max_brightness()).map(|v| v.to_string()) {
            properties.insert(KEY_KDB_MAX_BRIGHTNESS.into(), value);
        }
        if let Ok(value) = await_sync(self.keyboard.get_brightness()).map(|v| v.to_string()) {
            properties.insert(KEY_KDB_BRIGHTNESS.into(), value);
        }

        properties
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        None
    }
}

register_fallible_client!(Client, brightness);
