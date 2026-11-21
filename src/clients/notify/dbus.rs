use zbus::proxy;
#[proxy(interface = "org.freedesktop.Notifications", assume_defaults = true)]
pub trait Notifications {
    /// GetCapabilities method
    fn get_capabilities(&self) -> zbus::Result<Vec<String>>;

    /// Notify method
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: std::collections::HashMap<&str, &zbus::Variant<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    /// CloseNotification method
    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    /// GetServerInformation method
    fn get_server_information(&self) -> zbus::Result<(String, String, String, String)>;

    /// NotificationClosed signal
    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;

    /// ActionInvoked signal
    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: &str) -> zbus::Result<()>;

    /// ActivationToken signal
    #[zbus(signal)]
    fn activation_token(&self, id: u32, activation_token: &str) -> zbus::Result<()>;

    /// NotificationReplied signal
    #[zbus(signal)]
    fn notification_replied(&self, id: u32, text: &str) -> zbus::Result<()>;
}
