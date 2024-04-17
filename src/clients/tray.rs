use crate::register_fallible_client;
pub use system_tray::client::Client;

register_fallible_client!(Client, tray);
