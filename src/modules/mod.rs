/// Displays the current date and time.
///
/// A custom date/time format string can be set in the config.
///
/// Clicking the widget opens a popup containing the current time
/// with second-level precision and a calendar.
pub mod clock;
// pub mod focused;
// pub mod launcher;
// pub mod mpd;
// pub mod script;
// pub mod sysinfo;
// pub mod tray;
// pub mod workspaces;

use crate::config::BarPosition;
use color_eyre::Result;
use derive_builder::Builder;
use glib::IsA;
use gtk::gdk::Monitor;
use gtk::{Application, Widget};
use tokio::sync::mpsc;

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
    pub bar_position: &'a BarPosition,
    pub monitor: &'a Monitor,
    pub output_name: &'a str,
    pub module_name: &'a str,
}

#[derive(Clone)]
pub struct ModuleController<T> {
    rx: crossbeam_channel::Receiver<T>,
}

#[derive(Debug)]
pub enum ModuleUpdateEvent<T> {
    Update(T),
    TogglePopup,
}

pub struct WidgetContext<T> {
    pub id: String,
    pub tx: mpsc::Sender<ModuleUpdateEvent<T>>,
    pub widget_rx: glib::Receiver<T>,
    pub popup_rx: glib::Receiver<T>,
}

pub struct ModuleWidget<W: IsA<Widget>> {
    pub widget: W,
    pub popup: Option<gtk::Box>,
}

pub trait Module<W>
where
    W: IsA<Widget>,
{
    type Message;
    // type Sender = mpsc::Sender<Self::Message>;
    // type Receiver = glib::Receiver<Self::Message>;

    fn spawn_controller(
        &self,
        info: &ModuleInfo,
        tx: mpsc::Sender<ModuleUpdateEvent<Self::Message>>,
    ) -> Result<()>;

    fn into_widget(self, context: WidgetContext<Self::Message>) -> Result<ModuleWidget<W>>;
    // fn as_popup(&self, rx: glib::Receiver<T>) -> Option<Result<gtk::Box>>;

    // /// Consumes the module config
    // /// and produces a GTK widget of type `W`
    // fn into_widget(self, info: &ModuleInfo) -> Result<W>;
}

// pub fn setup_module<W, T>(
//     module: Box<dyn Module<W, T>>,
//     content: &gtk::Box,
//     info: ModuleInfo,
// ) -> Result<()>
// where
//     W: IsA<Widget>,
//     T: 'static + Clone + Send + Sync,
// {
//     let (w_tx, w_rx) = glib::MainContext::channel::<T>(glib::PRIORITY_DEFAULT);
//     let (p_tx, p_rx) = glib::MainContext::channel::<T>(glib::PRIORITY_DEFAULT);
//
//     let (tx, mut rx) = mpsc::channel::<T>(32);
//
//     module.spawn_controller(&info, tx);
//
//     let widget = module.into_widget(w_rx)?;
//     // let popup = module.as_popup(p_rx);
//
//     spawn(async move {
//         while let Some(ev) = rx.recv().await {
//             p_tx.send(ev.clone());
//             w_tx.send(ev);
//         }
//     });
//
//     content.add(&widget);
//     widget.set_widget_name(info.module_name);
//
//     Ok(())
// }
