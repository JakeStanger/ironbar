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
use derive_builder::Builder;
use glib::IsA;
use gtk::gdk::Monitor;
use gtk::{Application, Widget};
use tokio::sync::mpsc;
use crate::popup::ButtonGeometry;

#[derive(Clone)]
pub enum ModuleLocation {
    Left,
    Center,
    Right,
}

#[derive(Builder)]
pub struct ModuleInfo<'a> {
    pub app: &'a Application,
    pub location: ModuleLocation,
    pub bar_position: BarPosition,
    pub monitor: &'a Monitor,
    pub output_name: &'a str,
    pub module_name: &'a str,
}

#[derive(Debug)]
pub enum ModuleUpdateEvent<T> {
    /// Sends an update to the module UI
    Update(T),
    /// Toggles the open state of the popup.
    TogglePopup(ButtonGeometry),
    /// Force sets the popup open.
    /// Takes the button X position and width.
    OpenPopup(ButtonGeometry),
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
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        None
    }
}
