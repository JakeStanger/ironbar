/// Displays the current date and time.
///
/// A custom date/time format string can be set in the config.
///
/// Clicking the widget opens a popup containing the current time
/// with second-level precision and a calendar.
pub mod clock;
pub mod launcher;
pub mod mpd;
pub mod script;
pub mod sysinfo;
pub mod tray;
pub mod workspaces;

/// Shamelessly stolen from here:
/// <https://github.com/zeroeightysix/rustbar/blob/master/src/modules/module.rs>
use glib::IsA;
use gtk::{Application, Widget};
use serde::de::DeserializeOwned;
use serde_json::Value;
use crate::config::BarPosition;

#[derive(Clone)]
pub enum ModuleLocation {
    Left,
    Center,
    Right,
}

pub struct ModuleInfo<'a> {
    pub app: &'a Application,
    pub location: ModuleLocation,
    pub bar_position: &'a BarPosition,
    pub output_name: &'a str
}

pub trait Module<W>
where
    W: IsA<Widget>,
{
    /// Consumes the module config
    /// and produces a GTK widget of type `W`
    fn into_widget(self, info: &ModuleInfo) -> W;

    fn from_value(v: &Value) -> Box<Self>
    where
        Self: DeserializeOwned,
    {
        serde_json::from_value(v.clone()).unwrap()
    }
}
