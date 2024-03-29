use crate::register_client;
pub use system_tray::client::Client;

register_client!(Client, tray);
