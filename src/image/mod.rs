#[cfg(any(
    feature = "battery",
    feature = "bluetooth",
    feature = "clipboard",
    feature = "keyboard",
    feature = "launcher",
    feature = "menu",
    feature = "music",
    feature = "notifications",
    feature = "workspaces"
))]
mod gtk;
mod provider;

#[cfg(any(
    feature = "battery",
    feature = "bluetooth",
    feature = "clipboard",
    feature = "keyboard",
    feature = "launcher",
    feature = "menu",
    feature = "music",
    feature = "notifications",
    feature = "workspaces"
))]
pub use self::gtk::*;
pub use provider::Provider;
