mod dbus;

use self::dbus::NotificationsProxy;
use crate::clients::ClientResult;
use crate::register_fallible_client;
use std::collections::HashMap;
use std::sync::Arc;
use zbus::Connection;

/// Sends desktop notifications through the standard
/// `org.freedesktop.Notifications` service (mako, dunst, swaync...).
#[derive(Debug)]
pub struct Client {
    proxy: NotificationsProxy<'static>,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let dbus = Box::pin(Connection::session()).await?;
        let proxy = NotificationsProxy::new(&dbus).await?;

        Ok(Arc::new(Self { proxy }))
    }

    /// Sends a notification and returns its id.
    pub async fn notify(&self, summary: &str, body: &str, icon: &str) -> zbus::Result<u32> {
        self.proxy
            .notify(
                "Ironbar",      // app_name
                0,              // replaces_id: 0 = new notification
                icon,           // app_icon
                summary,        // summary
                body,           // body
                &[],            // actions
                HashMap::new(), // hints
                -1,             // expire_timeout: -1 = server default
            )
            .await
    }
}

register_fallible_client!(Client, notify);
