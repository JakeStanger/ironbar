#[cfg(feature = "clipboard")]
pub mod clipboard;
/// Displays the current date and time.
///
/// A custom date/time format string can be set in the config.
///
/// Clicking the widget opens a popup containing the current time
/// with second-level precision and a calendar.
#[cfg(feature = "clock")]
pub mod clock;
pub mod custom;
pub mod focused;
pub mod label;
pub mod launcher;
#[cfg(feature = "music")]
pub mod music;
pub mod script;
#[cfg(feature = "sys_info")]
pub mod sysinfo;
#[cfg(feature = "tray")]
pub mod tray;
#[cfg(feature = "workspaces")]
pub mod workspaces;

use crate::config::BarPosition;
use crate::popup::WidgetGeometry;
use color_eyre::Result;
use glib::IsA;
use gtk::gdk::Monitor;
use gtk::{Application, IconTheme, Widget};
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum ModuleLocation {
    Left,
    Center,
    Right,
}
pub struct ModuleInfo<'a> {
    pub app: &'a Application,
    pub location: ModuleLocation,
    pub bar_position: BarPosition,
    pub monitor: &'a Monitor,
    pub output_name: &'a str,
    pub icon_theme: &'a IconTheme,
}

#[derive(Debug)]
pub enum ModuleUpdateEvent<T> {
    /// Sends an update to the module UI
    Update(T),
    /// Toggles the open state of the popup.
    TogglePopup(WidgetGeometry),
    /// Force sets the popup open.
    /// Takes the button X position and width.
    OpenPopup(WidgetGeometry),
    /// Force sets the popup closed.
    ClosePopup,
}

pub struct WidgetContext<TSend, TReceive> {
    pub id: usize,
    pub tx: mpsc::Sender<ModuleUpdateEvent<TSend>>,
    pub controller_tx: mpsc::Sender<TReceive>,
    pub widget_rx: glib::Receiver<TSend>,
    pub popup_rx: glib::Receiver<TSend>,
}

pub struct ModuleWidget<W: IsA<Widget>> {
    pub widget: W,
    pub popup: Option<gtk::Box>,
}

pub trait Module<W>
where
    W: IsA<Widget>,
{
    type SendMessage;
    type ReceiveMessage;

    fn name() -> &'static str;

    fn spawn_controller(
        &self,
        info: &ModuleInfo,
        tx: mpsc::Sender<ModuleUpdateEvent<Self::SendMessage>>,
        rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()>;

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<W>>;

    fn into_popup(
        self,
        _tx: mpsc::Sender<Self::ReceiveMessage>,
        _rx: glib::Receiver<Self::SendMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        None
    }
}
