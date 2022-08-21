/// Displays the current date and time.
///
/// A custom date/time format string can be set in the config.
///
/// Clicking the widget opens a popup containing the current time
/// with second-level precision and a calendar.
pub mod clock;
pub mod focused;
pub mod launcher;
pub mod mpd;
pub mod script;
pub mod sysinfo;
pub mod tray;
pub mod workspaces;

use crate::config::BarPosition;
use color_eyre::Result;
/// Shamelessly stolen from here:
/// <https://github.com/zeroeightysix/rustbar/blob/master/src/modules/module.rs>
use glib::IsA;
use gtk::gdk::Monitor;
use gtk::{Application, Widget};

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
    pub monitor: &'a Monitor,
    pub output_name: &'a str,
}

pub trait Module<W>
where
    W: IsA<Widget>,
{
    /// Consumes the module config
    /// and produces a GTK widget of type `W`
    fn into_widget(self, info: &ModuleInfo) -> Result<W>;
}
