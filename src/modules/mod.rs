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
#[cfg(feature = "upower")]
pub mod upower;
#[cfg(feature = "workspaces")]
pub mod workspaces;

use crate::bridge_channel::BridgeChannel;
use crate::config::{BarPosition, CommonConfig, TransitionType};
use crate::popup::{Popup, WidgetGeometry};
use crate::{read_lock, send, write_lock};
use color_eyre::Result;
use glib::IsA;
use gtk::gdk::{EventMask, Monitor};
use gtk::prelude::*;
use gtk::{Application, EventBox, IconTheme, Orientation, Revealer, Widget};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::debug;

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

/// Creates a module and sets it up.
/// This setup includes widget/popup content and event channels.
pub fn create_module<TModule, TWidget, TSend, TRec>(
    module: TModule,
    id: usize,
    info: &ModuleInfo,
    popup: &Arc<RwLock<Popup>>,
) -> Result<TWidget>
where
    TModule: Module<TWidget, SendMessage = TSend, ReceiveMessage = TRec>,
    TWidget: IsA<Widget>,
    TSend: Clone + Send + 'static,
{
    let (w_tx, w_rx) = glib::MainContext::channel::<TSend>(glib::PRIORITY_DEFAULT);
    let (p_tx, p_rx) = glib::MainContext::channel::<TSend>(glib::PRIORITY_DEFAULT);

    let channel = BridgeChannel::<ModuleUpdateEvent<TSend>>::new();
    let (ui_tx, ui_rx) = mpsc::channel::<TRec>(16);

    module.spawn_controller(info, channel.create_sender(), ui_rx)?;

    let context = WidgetContext {
        id,
        widget_rx: w_rx,
        popup_rx: p_rx,
        tx: channel.create_sender(),
        controller_tx: ui_tx,
    };

    let name = TModule::name();

    let module_parts = module.into_widget(context, info)?;
    module_parts.widget.set_widget_name(name);

    let mut has_popup = false;
    if let Some(popup_content) = module_parts.popup {
        register_popup_content(popup, id, popup_content);
        has_popup = true;
    }

    setup_receiver(channel, w_tx, p_tx, popup.clone(), name, id, has_popup);

    Ok(module_parts.widget)
}

/// Registers the popup content with the popup.
fn register_popup_content(popup: &Arc<RwLock<Popup>>, id: usize, popup_content: gtk::Box) {
    write_lock!(popup).register_content(id, popup_content);
}

/// Sets up the bridge channel receiver
/// to pick up events from the controller, widget or popup.
///
/// Handles opening/closing popups
/// and communicating update messages between controllers and widgets/popups.
fn setup_receiver<TSend>(
    channel: BridgeChannel<ModuleUpdateEvent<TSend>>,
    w_tx: glib::Sender<TSend>,
    p_tx: glib::Sender<TSend>,
    popup: Arc<RwLock<Popup>>,
    name: &'static str,
    id: usize,
    has_popup: bool,
) where
    TSend: Clone + Send + 'static,
{
    // some rare cases can cause the popup to incorrectly calculate its size on first open.
    // we can fix that by just force re-rendering it on its first open.
    let mut has_popup_opened = false;

    channel.recv(move |ev| {
        match ev {
            ModuleUpdateEvent::Update(update) => {
                if has_popup {
                    send!(p_tx, update.clone());
                }

                send!(w_tx, update);
            }
            ModuleUpdateEvent::TogglePopup(geometry) => {
                debug!("Toggling popup for {} [#{}]", name, id);
                let popup = read_lock!(popup);
                if popup.is_visible() {
                    popup.hide();
                } else {
                    popup.show_content(id);
                    popup.show(geometry);

                    if !has_popup_opened {
                        popup.show_content(id);
                        popup.show(geometry);
                        has_popup_opened = true;
                    }
                }
            }
            ModuleUpdateEvent::OpenPopup(geometry) => {
                debug!("Opening popup for {} [#{}]", name, id);

                let popup = read_lock!(popup);
                popup.hide();
                popup.show_content(id);
                popup.show(geometry);

                if !has_popup_opened {
                    popup.show_content(id);
                    popup.show(geometry);
                    has_popup_opened = true;
                }
            }
            ModuleUpdateEvent::ClosePopup => {
                debug!("Closing popup for {} [#{}]", name, id);

                let popup = read_lock!(popup);
                popup.hide();
            }
        }

        Continue(true)
    });
}

/// Takes a widget and adds it into a new `gtk::EventBox`.
/// The event box container is returned.
pub fn wrap_widget<W: IsA<Widget>>(
    widget: &W,
    common: CommonConfig,
    orientation: Orientation,
) -> EventBox {
    let revealer = Revealer::builder()
        .transition_type(
            common
                .transition_type
                .as_ref()
                .unwrap_or(&TransitionType::SlideStart)
                .to_revealer_transition_type(orientation),
        )
        .transition_duration(common.transition_duration.unwrap_or(250))
        .build();

    revealer.add(widget);
    revealer.set_reveal_child(true);

    let container = EventBox::new();
    container.add_events(EventMask::SCROLL_MASK);
    container.add(&revealer);

    common.install(&container, &revealer);

    container
}
