use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;

use color_eyre::Result;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, Button, IconTheme, Orientation, Revealer, Widget};
use tokio::sync::{broadcast, mpsc};
use tracing::debug;

use crate::clients::{ClientResult, ProvidesClient, ProvidesFallibleClient};
use crate::config::{BarPosition, CommonConfig, TransitionType};
use crate::gtk_helpers::{IronbarGtkExt, WidgetGeometry};
use crate::popup::Popup;
use crate::{Ironbar, glib_recv_mpsc, send};

#[cfg(feature = "cairo")]
pub mod cairo;
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

#[cfg(feature = "custom")]
pub mod custom;
#[cfg(feature = "focused")]
pub mod focused;
#[cfg(feature = "keyboard")]
pub mod keyboard;

#[cfg(feature = "label")]
pub mod label;
#[cfg(feature = "launcher")]
pub mod launcher;
#[cfg(feature = "music")]
pub mod music;
#[cfg(feature = "network_manager")]
pub mod networkmanager;
#[cfg(feature = "notifications")]
pub mod notifications;

#[cfg(feature = "script")]
pub mod script;
#[cfg(feature = "sway")]
pub mod sway;
#[cfg(feature = "sys_info")]
pub mod sysinfo;
#[cfg(feature = "tray")]
pub mod tray;
#[cfg(feature = "upower")]
pub mod upower;
#[cfg(feature = "volume")]
pub mod volume;
#[cfg(feature = "workspaces")]
pub mod workspaces;

#[derive(Clone)]
pub enum ModuleLocation {
    Left,
    Center,
    Right,
}

#[derive(Clone)]
pub struct ModuleInfo<'a> {
    pub app: &'a Application,
    pub location: ModuleLocation,
    pub bar_position: BarPosition,
    pub monitor: &'a Monitor,
    pub output_name: &'a str,
    pub icon_theme: &'a IconTheme,
    pub icon_overrides: Arc<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub enum ModuleUpdateEvent<T: Clone> {
    /// Sends an update to the module UI.
    Update(T),
    /// Toggles the open state of the popup.
    /// Takes the button ID.
    TogglePopup(usize),
    /// Force sets the popup open.
    /// Takes the button ID.
    OpenPopup(usize),
    #[cfg(feature = "launcher")]
    OpenPopupAt(WidgetGeometry),
    /// Force sets the popup closed.
    ClosePopup,
}

pub struct WidgetContext<TSend, TReceive>
where
    TSend: Clone,
{
    pub id: usize,
    pub ironbar: Rc<Ironbar>,
    pub popup: Rc<Popup>,
    pub tx: mpsc::Sender<ModuleUpdateEvent<TSend>>,
    pub update_tx: broadcast::Sender<TSend>,
    pub controller_tx: mpsc::Sender<TReceive>,

    // TODO: Don't like this - need some serious refactoring to deal with it
    //  This is a hack to be able to pass data from module -> popup creation
    //  for custom widget only.
    pub button_id: usize,

    _update_rx: broadcast::Receiver<TSend>,
}

impl<TSend, TReceive> WidgetContext<TSend, TReceive>
where
    TSend: Clone,
{
    /// Gets client `T` from the context.
    ///
    /// This is a shorthand to avoid needing to go through
    /// `context.ironbar.clients`.
    pub fn client<T: ?Sized>(&self) -> Arc<T>
    where
        WidgetContext<TSend, TReceive>: ProvidesClient<T>,
    {
        ProvidesClient::provide(self)
    }

    pub fn try_client<T: ?Sized>(&self) -> ClientResult<T>
    where
        WidgetContext<TSend, TReceive>: ProvidesFallibleClient<T>,
    {
        ProvidesFallibleClient::try_provide(self)
    }

    /// Subscribes to events sent from this widget.
    pub fn subscribe(&self) -> broadcast::Receiver<TSend> {
        self.update_tx.subscribe()
    }
}

pub struct ModuleParts<W: IsA<Widget>> {
    pub widget: W,
    pub popup: Option<ModulePopupParts>,
}

impl<W: IsA<Widget>> ModuleParts<W> {
    fn new(widget: W, popup: Option<ModulePopupParts>) -> Self {
        Self { widget, popup }
    }

    pub fn setup_identifiers(&self, common: &CommonConfig) {
        if let Some(ref name) = common.name {
            self.widget.set_widget_name(name);

            if let Some(ref popup) = self.popup {
                popup.container.set_widget_name(&format!("popup-{name}"));
            }
        }

        if let Some(ref class) = common.class {
            // gtk counts classes with spaces as the same class
            for part in class.split(' ') {
                self.widget.style_context().add_class(part);
            }

            if let Some(ref popup) = self.popup {
                for part in class.split(' ') {
                    popup
                        .container
                        .style_context()
                        .add_class(&format!("popup-{part}"));
                }
            }
        }
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
    fn ensure_popup_id(&self) -> usize;
    fn try_popup_id(&self) -> Option<usize>;
    fn popup_id(&self) -> usize;
}

impl PopupButton for Button {
    /// Gets the popup ID associated with this button,
    /// or creates a new one if it does not exist.
    fn ensure_popup_id(&self) -> usize {
        if let Some(id) = self.try_popup_id() {
            id
        } else {
            let id = Ironbar::unique_id();
            self.set_tag("popup-id", id);
            id
        }
    }

    /// Gets the popup ID associated with this button, if there is one.
    /// Will return `None` if this is not a popup button.
    fn try_popup_id(&self) -> Option<usize> {
        self.get_tag("popup-id").copied()
    }

    /// Gets the popup ID associated with this button.
    /// This should only be called on buttons which are known to be associated with popups.
    ///
    /// # Panics
    /// Will panic if an ID has not been set.
    fn popup_id(&self) -> usize {
        self.try_popup_id().expect("id to exist")
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
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()>
    where
        <Self as Module<W>>::SendMessage: Clone;

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<W>>
    where
        <Self as Module<W>>::SendMessage: Clone;

    fn into_popup(
        self,
        _tx: mpsc::Sender<Self::ReceiveMessage>,
        _rx: broadcast::Receiver<Self::SendMessage>,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
        <Self as Module<W>>::SendMessage: Clone,
    {
        None
    }

    fn take_common(&mut self) -> CommonConfig;
}

pub trait ModuleFactory {
    fn create<TModule, TWidget, TSend, TRev>(
        &self,
        mut module: TModule,
        container: &gtk::Box,
        info: &ModuleInfo,
    ) -> Result<()>
    where
        TModule: Module<TWidget, SendMessage = TSend, ReceiveMessage = TRev>,
        TWidget: IsA<Widget>,
        TSend: Debug + Clone + Send + 'static,
    {
        let id = Ironbar::unique_id();
        let common = module.take_common();

        debug!("adding module {} (id: {})", TModule::name(), id);

        let (ui_tx, ui_rx) = mpsc::channel::<ModuleUpdateEvent<TSend>>(64);
        let (controller_tx, controller_rx) = mpsc::channel::<TRev>(64);

        let (tx, rx) = broadcast::channel(64);

        let context = WidgetContext {
            id,
            ironbar: self.ironbar().clone(),
            popup: self.popup().clone(),
            tx: ui_tx,
            update_tx: tx.clone(),
            controller_tx,
            _update_rx: rx,
            button_id: usize::MAX, // hack :(
        };

        module.spawn_controller(info, &context, controller_rx)?;

        let module_name = TModule::name();
        let instance_name = common
            .name
            .clone()
            .unwrap_or_else(|| module_name.to_string());

        let module_parts = module.into_widget(context, info)?;
        module_parts.widget.add_class("widget");
        module_parts.widget.add_class(module_name);

        if let Some(popup_content) = module_parts.popup.clone() {
            popup_content
                .container
                .style_context()
                .add_class(&format!("popup-{module_name}"));

            self.popup()
                .register_content(id, instance_name, popup_content);
        }

        self.setup_receiver(tx, ui_rx, module_name, id, common.disable_popup);

        module_parts.setup_identifiers(&common);

        let revealer = add_events(
            &module_parts.widget,
            common,
            info.bar_position.orientation(),
        );
        container.append(&revealer);

        Ok(())
    }

    fn setup_receiver<TSend>(
        &self,
        tx: broadcast::Sender<TSend>,
        rx: mpsc::Receiver<ModuleUpdateEvent<TSend>>,
        name: &'static str,
        id: usize,
        disable_popup: bool,
    ) where
        TSend: Debug + Clone + Send + 'static;

    fn ironbar(&self) -> &Rc<Ironbar>;
    fn popup(&self) -> &Rc<Popup>;
}

#[derive(Clone)]
pub struct BarModuleFactory {
    ironbar: Rc<Ironbar>,
    popup: Rc<Popup>,
}

impl BarModuleFactory {
    pub fn new(ironbar: Rc<Ironbar>, popup: Rc<Popup>) -> Self {
        Self { ironbar, popup }
    }
}

impl ModuleFactory for BarModuleFactory {
    fn setup_receiver<TSend>(
        &self,
        tx: broadcast::Sender<TSend>,
        rx: mpsc::Receiver<ModuleUpdateEvent<TSend>>,
        name: &'static str,
        id: usize,
        disable_popup: bool,
    ) where
        TSend: Debug + Clone + Send + 'static,
    {
        let popup = self.popup.clone();
        glib_recv_mpsc!(rx, ev => {
            match ev {
                ModuleUpdateEvent::Update(update) => {
                    send!(tx, update);
                }
                ModuleUpdateEvent::TogglePopup(button_id) if !disable_popup => {
                    debug!("Toggling popup for {} [#{}] (button id: {button_id})", name, id);
                    if popup.visible() && popup.current_widget().unwrap_or_default() == id {
                        popup.hide();
                    } else {
                        popup.show(id, button_id);
                    }
                }
                ModuleUpdateEvent::OpenPopup(button_id) if !disable_popup => {
                    debug!("Opening popup for {} [#{}] (button id: {button_id})", name, id);
                    popup.hide();
                    popup.show(id, button_id);
                }
                #[cfg(feature = "launcher")]
                ModuleUpdateEvent::OpenPopupAt(geometry) if !disable_popup => {
                    debug!("Opening popup for {} [#{}]", name, id);

                    popup.hide();
                    popup.show_at(id, geometry);
                }
                ModuleUpdateEvent::ClosePopup if !disable_popup => {
                    debug!("Closing popup for {} [#{}]", name, id);
                    popup.hide();
                },
                _ => {}
            }
        });
    }

    fn ironbar(&self) -> &Rc<Ironbar> {
        &self.ironbar
    }

    fn popup(&self) -> &Rc<Popup> {
        &self.popup
    }
}

#[derive(Clone)]
pub struct PopupModuleFactory {
    ironbar: Rc<Ironbar>,
    popup: Rc<Popup>,
    button_id: usize,
}

impl PopupModuleFactory {
    pub fn new(ironbar: Rc<Ironbar>, popup: Rc<Popup>, button_id: usize) -> Self {
        Self {
            ironbar,
            popup,
            button_id,
        }
    }
}

impl ModuleFactory for PopupModuleFactory {
    fn setup_receiver<TSend>(
        &self,
        tx: broadcast::Sender<TSend>,
        rx: mpsc::Receiver<ModuleUpdateEvent<TSend>>,
        name: &'static str,
        id: usize,
        disable_popup: bool,
    ) where
        TSend: Debug + Clone + Send + 'static,
    {
        let popup = self.popup.clone();
        let button_id = self.button_id;
        glib_recv_mpsc!(rx, ev => {
            match ev {
                ModuleUpdateEvent::Update(update) => {
                    send!(tx, update);
                }
                ModuleUpdateEvent::TogglePopup(_) if !disable_popup => {
                    debug!("Toggling popup for {} [#{}] (button id: {button_id})", name, id);
                    if popup.visible() && popup.current_widget().unwrap_or_default() == id {
                        popup.hide();
                    } else {
                        popup.show(id, button_id);
                    }
                }
                ModuleUpdateEvent::OpenPopup(_) if !disable_popup => {
                    debug!("Opening popup for {} [#{}] (button id: {button_id})", name, id);
                    popup.hide();
                    popup.show(id, button_id);
                }
                #[cfg(feature = "launcher")]
                ModuleUpdateEvent::OpenPopupAt(geometry) if !disable_popup => {
                    debug!("Opening popup for {} [#{}]", name, id);

                    popup.hide();
                    popup.show_at(id, geometry);
                }
                ModuleUpdateEvent::ClosePopup if !disable_popup => {
                    debug!("Closing popup for {} [#{}]", name, id);
                    popup.hide();
                },
                _ => {}
            }
        });
    }

    fn ironbar(&self) -> &Rc<Ironbar> {
        &self.ironbar
    }

    fn popup(&self) -> &Rc<Popup> {
        &self.popup
    }
}

#[derive(Clone)]
pub enum AnyModuleFactory {
    Bar(BarModuleFactory),
    Popup(PopupModuleFactory),
}

impl ModuleFactory for AnyModuleFactory {
    fn setup_receiver<TSend>(
        &self,
        tx: broadcast::Sender<TSend>,
        rx: mpsc::Receiver<ModuleUpdateEvent<TSend>>,
        name: &'static str,
        id: usize,
        disable_popup: bool,
    ) where
        TSend: Debug + Clone + Send + 'static,
    {
        match self {
            AnyModuleFactory::Bar(bar) => bar.setup_receiver(tx, rx, name, id, disable_popup),
            AnyModuleFactory::Popup(popup) => popup.setup_receiver(tx, rx, name, id, disable_popup),
        }
    }

    fn ironbar(&self) -> &Rc<Ironbar> {
        match self {
            AnyModuleFactory::Bar(bar) => bar.ironbar(),
            AnyModuleFactory::Popup(popup) => popup.ironbar(),
        }
    }

    fn popup(&self) -> &Rc<Popup> {
        match self {
            AnyModuleFactory::Bar(bar) => bar.popup(),
            AnyModuleFactory::Popup(popup) => popup.popup(),
        }
    }
}

impl From<BarModuleFactory> for AnyModuleFactory {
    fn from(value: BarModuleFactory) -> Self {
        Self::Bar(value)
    }
}

impl From<PopupModuleFactory> for AnyModuleFactory {
    fn from(value: PopupModuleFactory) -> Self {
        Self::Popup(value)
    }
}

/// Takes a widget and adds event listeners and the revealer.
/// Returns the revealer.
pub fn add_events<W: IsA<Widget>>(
    widget: &W,
    common: CommonConfig,
    orientation: Orientation,
) -> Revealer {
    let transition_type = common
        .transition_type
        .as_ref()
        .unwrap_or(&TransitionType::SlideStart)
        .to_revealer_transition_type(orientation);

    let revealer = Revealer::builder()
        .transition_type(transition_type)
        .transition_duration(common.transition_duration.unwrap_or(250))
        .build();

    revealer.set_child(Some(widget));
    revealer.set_reveal_child(true);

    common.install_events(widget, &revealer);
    revealer
}
