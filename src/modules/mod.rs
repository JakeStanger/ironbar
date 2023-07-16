use std::sync::{Arc, RwLock};

use color_eyre::Result;
use glib::IsA;
use gtk::gdk::{EventMask, Monitor};
use gtk::prelude::*;
use gtk::{Application, Button, EventBox, IconTheme, Orientation, Revealer, Widget};
use tokio::sync::mpsc;
use tracing::debug;

use crate::bridge_channel::BridgeChannel;
use crate::config::{BarPosition, CommonConfig, TransitionType};
use crate::gtk_helpers::{IronbarGtkExt, WidgetGeometry};
use crate::popup::Popup;
use crate::{send, write_lock};

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
    /// Sends an update to the module UI.
    Update(T),
    /// Toggles the open state of the popup.
    /// Takes the button ID.
    TogglePopup(usize),
    /// Force sets the popup open.
    /// Takes the button ID.
    OpenPopup(usize),
    OpenPopupAt(WidgetGeometry),
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

pub struct ModuleParts<W: IsA<Widget>> {
    pub widget: W,
    pub popup: Option<ModulePopupParts>,
}

impl<W: IsA<Widget>> ModuleParts<W> {
    fn new(widget: W, popup: Option<ModulePopupParts>) -> Self {
        Self { widget, popup }
    }
}

#[derive(Debug, Clone)]
pub struct ModulePopupParts {
    /// The popup container, with all its contents
    pub container: gtk::Box,
    /// An array of buttons which can be used for opening the popup.
    /// For most modules, this will only be a single button.
    /// For some advanced modules, such as `Launcher`, this is all item buttons.
    pub buttons: Vec<Button>,
}

pub trait ModulePopup {
    fn into_popup_parts(self, buttons: Vec<&Button>) -> Option<ModulePopupParts>;
    fn into_popup_parts_owned(self, buttons: Vec<Button>) -> Option<ModulePopupParts>;
}

impl ModulePopup for Option<gtk::Box> {
    fn into_popup_parts(self, buttons: Vec<&Button>) -> Option<ModulePopupParts> {
        self.into_popup_parts_owned(buttons.into_iter().cloned().collect())
    }

    fn into_popup_parts_owned(self, buttons: Vec<Button>) -> Option<ModulePopupParts> {
        self.map(|container| ModulePopupParts { container, buttons })
    }
}

pub trait PopupButton {
    fn popup_id(&self) -> usize;
}

impl PopupButton for Button {
    /// Gets the popup ID associated with this button.
    /// This should only be called on buttons which are known to be associated with popups.
    ///
    /// # Panics
    /// Will panic if an ID has not been set.
    fn popup_id(&self) -> usize {
        *self.get_tag("popup-id").expect("data to exist")
    }
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
    ) -> Result<ModuleParts<W>>;

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
    name: Option<String>,
    info: &ModuleInfo,
    popup: &Arc<RwLock<Popup>>,
) -> Result<ModuleParts<TWidget>>
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

    let module_name = TModule::name();
    let instance_name = name.unwrap_or_else(|| module_name.to_string());

    let module_parts = module.into_widget(context, info)?;
    module_parts.widget.style_context().add_class(module_name);

    let has_popup = if let Some(popup_content) = module_parts.popup.clone() {
        popup_content
            .container
            .style_context()
            .add_class(&format!("popup-{module_name}"));

        register_popup_content(popup, id, instance_name, popup_content);
        true
    } else {
        false
    };

    setup_receiver(
        channel,
        w_tx,
        p_tx,
        popup.clone(),
        module_name,
        id,
        has_popup,
    );

    Ok(module_parts)
}

/// Registers the popup content with the popup.
fn register_popup_content(
    popup: &Arc<RwLock<Popup>>,
    id: usize,
    name: String,
    popup_content: ModulePopupParts,
) {
    write_lock!(popup).register_content(id, name, popup_content);
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
            ModuleUpdateEvent::TogglePopup(button_id) => {
                debug!("Toggling popup for {} [#{}]", name, id);
                let mut popup = write_lock!(popup);
                if popup.is_visible() {
                    popup.hide();
                } else {
                    popup.show(id, button_id);

                    // force re-render on initial open to try and fix size issue
                    if !has_popup_opened {
                        popup.show(id, button_id);
                        has_popup_opened = true;
                    }
                }
            }
            ModuleUpdateEvent::OpenPopup(button_id) => {
                debug!("Opening popup for {} [#{}]", name, id);

                let mut popup = write_lock!(popup);
                popup.hide();
                popup.show(id, button_id);

                // force re-render on initial open to try and fix size issue
                if !has_popup_opened {
                    popup.show(id, button_id);
                    has_popup_opened = true;
                }
            }
            ModuleUpdateEvent::OpenPopupAt(geometry) => {
                debug!("Opening popup for {} [#{}]", name, id);

                let mut popup = write_lock!(popup);
                popup.hide();
                popup.show_at(id, geometry);

                // force re-render on initial open to try and fix size issue
                if !has_popup_opened {
                    popup.show_at(id, geometry);
                    has_popup_opened = true;
                }
            }
            ModuleUpdateEvent::ClosePopup => {
                debug!("Closing popup for {} [#{}]", name, id);

                let mut popup = write_lock!(popup);
                popup.hide();
            }
        }

        Continue(true)
    });
}

pub fn set_widget_identifiers<TWidget: IsA<Widget>>(
    widget_parts: &ModuleParts<TWidget>,
    common: &CommonConfig,
) {
    if let Some(ref name) = common.name {
        widget_parts.widget.set_widget_name(name);

        if let Some(ref popup) = widget_parts.popup {
            popup.container.set_widget_name(&format!("popup-{name}"));
        }
    }

    if let Some(ref class) = common.class {
        // gtk counts classes with spaces as the same class
        for part in class.split(' ') {
            widget_parts.widget.style_context().add_class(part);
        }

        if let Some(ref popup) = widget_parts.popup {
            for part in class.split(' ') {
                popup
                    .container
                    .style_context()
                    .add_class(&format!("popup-{part}"));
            }
        }
    }
}

/// Takes a widget and adds it into a new `gtk::EventBox`.
/// The event box container is returned.
pub fn wrap_widget<W: IsA<Widget>>(
    widget: &W,
    common: CommonConfig,
    orientation: Orientation,
) -> EventBox {
    let transition_type = common
        .transition_type
        .as_ref()
        .unwrap_or(&TransitionType::SlideStart)
        .to_revealer_transition_type(orientation);

    let revealer = Revealer::builder()
        .transition_type(transition_type)
        .transition_duration(common.transition_duration.unwrap_or(250))
        .build();

    revealer.add(widget);
    revealer.set_reveal_child(true);

    let container = EventBox::new();
    container.add_events(EventMask::SCROLL_MASK);
    container.add(&revealer);

    common.install_events(&container, &revealer);

    container
}
