mod icon;
mod interface;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::tray;
use crate::config::{CommonConfig, ModuleOrientation, default};
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{lock, module_impl, spawn};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{IconTheme, Orientation};
use interface::TrayMenu;
use serde::Deserialize;
use std::collections::HashMap;
use system_tray::client::Event;
use system_tray::client::{ActivateRequest, UpdateEvent};
use tokio::sync::mpsc;
use tracing::{debug, error, trace, warn};

/// Icon configuration for tray items
struct IconConfig {
    theme: IconTheme,
    size: u32,
    prefer_theme: bool,
}

/// Reserved tray click actions
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReservedTrayAction {
    /// Open the tray icon's popup menu
    Menu,
    /// Trigger the tray icon's default (primary) action
    Default,
    /// Trigger the tray icon's secondary action
    Secondary,
    /// Do nothing
    None,
}

/// Action to perform when clicking on a tray icon
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TrayClickAction {
    /// Reserved action
    Reserved(ReservedTrayAction),
    /// Run a custom shell command
    Custom(String),
}

/// Click action handlers for tray icons
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct TrayClickHandlers {
    /// Action to perform on left-click.
    ///
    /// **Valid options**: `menu`, `default`, `secondary`, `none`, or any custom shell command
    /// <br>
    /// **Default**: `default` (current behaviour, for backwards compatibility)
    ///
    /// Custom commands support the following placeholders:
    /// - `{name}` - The tray item's identifier/name
    /// - `{title}` - The tray item's title (if available)
    /// - `{icon}` - The tray item's icon name (if available)
    /// - `{address}` - The tray item's internal address
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_left = "menu" }
    /// { on_click_left = "notify-send 'Clicked {name}'" }
    /// { on_click_left = "if [ '{name}' = 'copyq' ]; then copyq toggle; fi" }
    /// ```
    #[cfg_attr(feature = "extras", schemars(extend("default" = "'default'")))]
    on_click_left: TrayClickAction,

    /// Action to perform on right-click.
    ///
    /// **Valid options**: `menu`, `default`, `secondary`, `none`, or any custom shell command
    /// <br>
    /// **Default**: `menu`
    #[cfg_attr(feature = "extras", schemars(extend("default" = "'menu'")))]
    on_click_right: TrayClickAction,

    /// Action to perform on middle-click.
    ///
    /// **Valid options**: `menu`, `default`, `secondary`, `none`, or any custom shell command
    /// <br>
    /// **Default**: `none`
    on_click_middle: TrayClickAction,

    /// Action to perform on double-left-click.
    ///
    /// **Valid options**: `menu`, `default`, `secondary`, `none`, or any custom shell command
    /// <br>
    /// **Default**: `none`
    on_click_left_double: TrayClickAction,

    /// Action to perform on double-right-click.
    ///
    /// **Valid options**: `menu`, `default`, `secondary`, `none`, or any custom shell command
    /// <br>
    /// **Default**: `none`
    on_click_right_double: TrayClickAction,

    /// Action to perform on double-middle-click.
    ///
    /// **Valid options**: `menu`, `default`, `secondary`, `none`, or any custom shell command
    /// <br>
    /// **Default**: `none`
    on_click_middle_double: TrayClickAction,
}

impl Default for TrayClickHandlers {
    fn default() -> Self {
        Self {
            on_click_left: TrayClickAction::Reserved(ReservedTrayAction::Default),
            on_click_right: TrayClickAction::Reserved(ReservedTrayAction::Menu),
            on_click_middle: TrayClickAction::Reserved(ReservedTrayAction::None),
            on_click_left_double: TrayClickAction::Reserved(ReservedTrayAction::None),
            on_click_right_double: TrayClickAction::Reserved(ReservedTrayAction::None),
            on_click_middle_double: TrayClickAction::Reserved(ReservedTrayAction::None),
        }
    }
}

impl TrayClickAction {
    /// Returns true if this action is not None
    pub fn is_actionable(&self) -> bool {
        !matches!(self, Self::Reserved(ReservedTrayAction::None))
    }
}

impl Default for TrayClickAction {
    fn default() -> Self {
        Self::Reserved(ReservedTrayAction::None)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct TrayModule {
    /// Requests that icons from the theme be used over the item-provided item.
    /// Most items only provide one or the other so this will have no effect in most circumstances.
    ///
    /// **Default**: `true`
    prefer_theme_icons: bool,

    /// Size in pixels to display the tray icons as.
    ///
    /// **Default**: `16`
    icon_size: u32,

    /// The direction in which to pack tray icons.
    ///
    /// **Valid options**: `horizontal`, `vertical`
    /// <br>
    /// **Default**: `horizontal` for horizontal bars, `vertical` for vertical bars
    #[cfg_attr(feature = "extras", schemars(extend("default" = "[matches bar orientation]")))]
    direction: Option<ModuleOrientation>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,

    /// Click action handlers for tray icons.
    ///
    /// Click actions can be one of the following built-in actions, or any custom shell command:
    ///
    /// **Built-in actions:**
    /// - `menu` - Opens the tray icon's popup menu
    /// - `default` - Triggers the tray icon's default (primary) action
    /// - `secondary` - Triggers the tray icon's secondary action
    /// - `none` - Do nothing
    ///
    /// **Custom commands:**
    ///
    /// Any other string is treated as a custom shell command. Custom commands support the following placeholders:
    /// - `{name}` - The tray item's identifier/name
    /// - `{title}` - The tray item's title (if available)
    /// - `{icon}` - The tray item's icon name (if available)
    /// - `{address}` - The tray item's internal address
    ///
    /// **Examples:**
    ///
    /// ```corn
    /// {
    ///   type = "tray"
    ///   on_click_left = "menu"
    ///   on_click_left_double = "default"
    /// }
    /// ```
    ///
    /// To run custom commands based on which tray item was clicked:
    /// ```corn
    /// {
    ///   type = "tray"
    ///   on_click_left = "notify-send 'Clicked {name}'"
    ///   on_click_middle = "if [ '{name}' = 'copyq' ]; then copyq toggle; fi"
    /// }
    /// ```
    #[serde(flatten)]
    click_handlers: TrayClickHandlers,
}

impl Default for TrayModule {
    fn default() -> Self {
        Self {
            prefer_theme_icons: true,
            icon_size: default::IconSize::Tiny as u32,
            direction: None,
            click_handlers: TrayClickHandlers::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

pub enum UiEvent {
    Menu(bool),
    Activate(ActivateRequest),
}

impl Module<gtk::Box> for TrayModule {
    type SendMessage = Event;
    type ReceiveMessage = UiEvent;

    module_impl!("tray");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();

        let client = context.try_client::<tray::Client>()?;
        let mut tray_rx = client.subscribe();

        let initial_items = {
            let items = client.items();
            lock!(items).clone()
        };

        // listen to tray updates
        spawn({
            let tx = tx.clone();
            async move {
                let mut known_ids = std::collections::HashSet::new();

                for (key, (item, menu)) in initial_items {
                    known_ids.insert(key.clone());
                    tx.send_update(Event::Add(key.clone(), item.into())).await;

                    if let Some(menu) = menu.clone() {
                        tx.send_update(Event::Update(key, UpdateEvent::Menu(menu)))
                            .await;
                    }
                }

                while let Ok(message) = tray_rx.recv().await {
                    match &message {
                        Event::Add(address, _) => {
                            if !known_ids.insert(address.clone()) {
                                debug!("Skipping duplicate tray item: {address}");
                                continue;
                            }
                        }
                        Event::Remove(address) => {
                            known_ids.remove(address);
                        }
                        _ => {}
                    }

                    tx.send_update(message).await;
                }
            }
        });

        // send tray commands
        spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    UiEvent::Menu(open) => {
                        tx.send_expect(ModuleUpdateEvent::LockVisible(open)).await;
                    }
                    UiEvent::Activate(action) => {
                        debug!("activating: {action:?}");
                        if let Err(err) = client.activate(action).await {
                            error!("{err:?}");
                        }
                        trace!("end activation");
                    }
                }
            }

            Ok::<_, Report>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let orientation = self
            .direction
            .map_or(info.bar_position.orientation(), Orientation::from);

        // We use a `Box` here instead of the (supposedly correct) `MenuBar`
        // as the latter has issues on Sway with menus focus-stealing from the bar.
        let container = gtk::Box::new(orientation, 0);

        {
            let container = container.clone();
            let mut menus = HashMap::new();
            let activated_channel = context.controller_tx.clone();

            let provider = context.ironbar.image_provider();
            let icon_config = IconConfig {
                theme: provider.icon_theme().clone(),
                size: self.icon_size,
                prefer_theme: self.prefer_theme_icons,
            };

            // listen for UI updates
            let click_handlers = self.click_handlers.clone();

            context.subscribe().recv_glib((), move |(), update| {
                on_update(
                    update,
                    &container,
                    &mut menus,
                    &icon_config,
                    &activated_channel,
                    &click_handlers,
                );
            });
        };

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }

    fn into_popup(
        self,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
        <Self as Module<gtk::Box>>::SendMessage: Clone,
    {
        Some(gtk::Box::new(info.bar_position.orientation(), 0))
    }
}

/// Handles UI updates as callback,
/// getting the diff since the previous update and applying it to the menu.
fn on_update(
    update: Event,
    container: &gtk::Box,
    menus: &mut HashMap<Box<str>, TrayMenu>,
    icon_config: &IconConfig,
    activated_channel: &mpsc::Sender<UiEvent>,
    click_handlers: &TrayClickHandlers,
) {
    match update {
        Event::Add(address, item) => {
            debug!("Received new tray item at '{address}': {item:?}");

            let mut menu_item =
                TrayMenu::new(&address, *item, activated_channel.clone(), click_handlers);

            let x: Option<&gtk::Widget> = None;
            container.insert_child_after(&menu_item.widget, x);

            if let Ok(image) = icon::get_image(
                &menu_item,
                icon_config.size,
                icon_config.prefer_theme,
                &icon_config.theme,
            ) {
                menu_item.set_image(&image);
            } else {
                let label = menu_item.title.clone().unwrap_or(address.clone());
                menu_item.set_label(&label);
            }

            menus.insert(address.into(), menu_item);
        }
        Event::Update(address, update) => {
            debug!("Received tray update for '{address}'");
            trace!("Tray update for '{address}: {update:?}'");

            let Some(menu_item) = menus.get_mut(address.as_str()) else {
                error!("Attempted to update menu at '{address}' but could not find it");
                return;
            };

            match update {
                UpdateEvent::AttentionIcon(_icon) => {
                    warn!("received unimplemented NewAttentionIcon event");
                }
                UpdateEvent::Icon {
                    icon_name,
                    icon_pixmap,
                } => {
                    let name_changed = icon_name.as_ref() != menu_item.icon_name();
                    let pixmap_changed = icon_pixmap != menu_item.icon_pixmap;

                    if name_changed || pixmap_changed {
                        menu_item.icon_pixmap = icon_pixmap;
                        menu_item.set_icon_name(icon_name);

                        match icon::get_image(
                            menu_item,
                            icon_config.size,
                            icon_config.prefer_theme,
                            &icon_config.theme,
                        ) {
                            Ok(image) => menu_item.set_image(&image),
                            Err(e) => {
                                error!("error loading icon: {e}");
                                menu_item.show_label();
                            }
                        }
                    }
                }
                UpdateEvent::OverlayIcon(_icon) => {
                    warn!("received unimplemented NewOverlayIcon event");
                }
                UpdateEvent::Status(status) => {
                    menu_item.set_status(status);
                }
                UpdateEvent::Title(title) => {
                    if let Some(label_widget) = menu_item.label_widget() {
                        label_widget.set_label(&title.unwrap_or_default());
                    }
                }
                UpdateEvent::Tooltip(tooltip) => {
                    menu_item.set_tooltip(tooltip);
                }
                UpdateEvent::MenuConnect(menu) => {
                    menu_item.set_menu(&menu);
                }
                UpdateEvent::Menu(menu) => {
                    menu_item.set_menu_widget(&menu);
                }
                UpdateEvent::MenuDiff(diff) => {
                    trace!("received menu diff {diff:?}");
                }
            }
        }
        Event::Remove(address) => {
            debug!("Removing tray item at '{address}'");

            if let Some(menu) = menus.get(address.as_str()) {
                container.remove(&menu.widget);
            }
        }
    }
}
