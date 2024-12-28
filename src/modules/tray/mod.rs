mod icon;
mod interface;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::tray;
use crate::config::{CommonConfig, ModuleOrientation};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
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

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TrayModule {
    /// Requests that icons from the theme be used over the item-provided item.
    /// Most items only provide one or the other so this will have no effect in most circumstances.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    prefer_theme_icons: bool,

    /// Size in pixels to display the tray icons as.
    ///
    /// **Default**: `16`
    #[serde(default = "default_icon_size")]
    icon_size: u32,

    /// The direction in which to pack tray icons.
    ///
    /// **Valid options**: `horizontal`, `vertical`
    /// <br>
    /// **Default**: `horizontal` for horizontal bars, `vertical` for vertical bars
    #[serde(default)]
    direction: Option<ModuleOrientation>,
    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_icon_size() -> u32 {
    16
}

impl Module<gtk::Box> for TrayModule {
    type SendMessage = Event;
    type ReceiveMessage = ActivateRequest;

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

        let initial_items = lock!(client.items()).clone();

        // listen to tray updates
        spawn(async move {
            for (key, (item, menu)) in initial_items {
                tx.send_update(Event::Add(key.clone(), item.into())).await;

                if let Some(menu) = menu.clone() {
                    tx.send_update(Event::Update(key, UpdateEvent::Menu(menu)))
                        .await;
                }
            }

            while let Ok(message) = tray_rx.recv().await {
                tx.send_update(message).await;
            }
        });

        // send tray commands
        spawn(async move {
            while let Some(cmd) = rx.recv().await {
                if let Err(err) = client.activate(cmd).await {
                    error!("{err:?}");
                };
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
            .map(Orientation::from)
            .unwrap_or(info.bar_position.orientation());

        // We use a `Box` here instead of the (supposedly correct) `MenuBar`
        // as the latter has issues on Sway with menus focus-stealing from the bar.
        //
        // Each widget is wrapped in an EventBox, copying what Waybar does here.
        let container = gtk::Box::new(orientation, 10);

        {
            let container = container.clone();
            let mut menus = HashMap::new();
            let icon_theme = info.icon_theme.clone();

            // listen for UI updates
            context.subscribe().recv_glib(move |update| {
                on_update(
                    update,
                    &container,
                    &mut menus,
                    &icon_theme,
                    self.icon_size,
                    self.prefer_theme_icons,
                );
            });
        };

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}

/// Handles UI updates as callback,
/// getting the diff since the previous update and applying it to the menu.
fn on_update(
    update: Event,
    container: &gtk::Box,
    menus: &mut HashMap<Box<str>, TrayMenu>,
    icon_theme: &IconTheme,
    icon_size: u32,
    prefer_icons: bool,
) {
    match update {
        Event::Add(address, item) => {
            debug!("Received new tray item at '{address}': {item:?}");

            let mut menu_item = TrayMenu::new(&address, *item);
            container.pack_start(&menu_item.event_box, true, true, 0);

            if let Ok(image) = icon::get_image(&menu_item, icon_theme, icon_size, prefer_icons) {
                menu_item.set_image(&image);
            } else {
                let label = menu_item.title.clone().unwrap_or(address.clone());
                menu_item.set_label(&label);
            };

            menu_item.event_box.show();
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
                UpdateEvent::Icon(icon) => {
                    if icon.as_ref() != menu_item.icon_name() {
                        menu_item.set_icon_name(icon);
                        match icon::get_image(menu_item, icon_theme, icon_size, prefer_icons) {
                            Ok(image) => menu_item.set_image(&image),
                            Err(_) => menu_item.show_label(),
                        };
                    }
                }
                UpdateEvent::OverlayIcon(_icon) => {
                    warn!("received unimplemented NewOverlayIcon event");
                }
                UpdateEvent::Status(_status) => {
                    warn!("received unimplemented NewStatus event");
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
                    let menu = system_tray::gtk_menu::Menu::new(&address, &menu);
                    menu_item.set_menu_widget(menu);
                }
                UpdateEvent::Menu(_) | UpdateEvent::MenuDiff(_) => {}
            }
        }
        Event::Remove(address) => {
            debug!("Removing tray item at '{address}'");

            if let Some(menu) = menus.get(address.as_str()) {
                container.remove(&menu.event_box);
            }
        }
    };
}
