mod diff;
mod icon;
mod interface;

use crate::clients::tray;
use crate::config::CommonConfig;
use crate::modules::tray::diff::get_diffs;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, lock, send_async, spawn};
use color_eyre::{Report, Result};
use gtk::{prelude::*, PackDirection};
use gtk::{IconTheme, MenuBar};
use interface::TrayMenu;
use serde::Deserialize;
use std::collections::HashMap;
use system_tray::client::Event;
use system_tray::client::{ActivateRequest, UpdateEvent};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

#[derive(Debug, Deserialize, Clone)]
pub struct TrayModule {
    #[serde(default = "default_icon_size")]
    icon_size: u32,

    #[serde(default, deserialize_with = "deserialize_orientation")]
    direction: Option<PackDirection>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_icon_size() -> u32 {
    16
}

fn deserialize_orientation<'de, D>(deserializer: D) -> Result<Option<PackDirection>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(|v| match v.as_str() {
            "left_to_right" => Ok(PackDirection::Ltr),
            "right_to_left" => Ok(PackDirection::Rtl),
            "top_to_bottom" => Ok(PackDirection::Ttb),
            "bottom_to_top" => Ok(PackDirection::Btt),
            _ => Err(serde::de::Error::custom("invalid value for orientation")),
        })
        .transpose()
}

impl Module<MenuBar> for TrayModule {
    type SendMessage = Event;
    type ReceiveMessage = ActivateRequest;

    fn name() -> &'static str {
        "tray"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();

        let client = context.client::<tray::Client>();
        let mut tray_rx = client.subscribe();

        let initial_items = lock!(client.items()).clone();

        // listen to tray updates
        spawn(async move {
            for (key, (item, menu)) in initial_items.into_iter() {
                send_async!(
                    tx,
                    ModuleUpdateEvent::Update(Event::Add(key.clone(), item.into()))
                );

                if let Some(menu) = menu.clone() {
                    send_async!(
                        tx,
                        ModuleUpdateEvent::Update(Event::Update(key, UpdateEvent::Menu(menu)))
                    );
                }
            }

            while let Ok(message) = tray_rx.recv().await {
                send_async!(tx, ModuleUpdateEvent::Update(message))
            }
        });

        // send tray commands
        spawn(async move {
            while let Some(cmd) = rx.recv().await {
                client.activate(cmd).await?;
            }

            Ok::<_, Report>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<MenuBar>> {
        let container = MenuBar::new();

        let direction = self.direction.unwrap_or(
            if info.bar_position.orientation() == gtk::Orientation::Vertical {
                PackDirection::Ttb
            } else {
                PackDirection::Ltr
            },
        );

        container.set_pack_direction(direction);
        container.set_child_pack_direction(direction);

        {
            let container = container.clone();
            let mut menus = HashMap::new();
            let icon_theme = info.icon_theme.clone();

            // listen for UI updates
            glib_recv!(context.subscribe(), update =>
                on_update(update, &container, &mut menus, &icon_theme, self.icon_size, self.prefer_theme_icons, &context.controller_tx)
            );
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
    container: &MenuBar,
    menus: &mut HashMap<Box<str>, TrayMenu>,
    icon_theme: &IconTheme,
    icon_size: u32,
    prefer_icons: bool,
    tx: &mpsc::Sender<ActivateRequest>,
) {
    match update {
        Event::Add(address, item) => {
            debug!("Received new tray item at '{address}': {item:?}");

            let mut menu_item = TrayMenu::new(tx.clone(), address.clone(), *item);
            container.add(&menu_item.widget);

            match icon::get_image(&menu_item, icon_theme, icon_size, prefer_icons) {
                Ok(image) => menu_item.set_image(&image),
                Err(_) => {
                    let label = menu_item.title.clone().unwrap_or(address.clone());
                    menu_item.set_label(&label)
                }
            };

            menu_item.widget.show();
            menus.insert(address.into(), menu_item);
        }
        Event::Update(address, update) => {
            debug!("Received tray update for '{address}': {update:?}");

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
                        match icon::get_image(menu_item, icon_theme, icon_size, prefer_icons) {
                            Ok(image) => menu_item.set_image(&image),
                            Err(_) => menu_item.show_label(),
                        };
                    }

                    menu_item.set_icon_name(icon);
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
                // UpdateEvent::Tooltip(_tooltip) => {
                //     warn!("received unimplemented NewAttentionIcon event");
                // }
                UpdateEvent::Menu(menu) => {
                    debug!("received new menu for '{}'", address);

                    let diffs = get_diffs(menu_item.state(), &menu.submenus);

                    menu_item.apply_diffs(diffs);
                    menu_item.set_state(menu.submenus);
                }
            }
        }
        Event::Remove(address) => {
            debug!("Removing tray item at '{address}'");

            if let Some(menu) = menus.get(address.as_str()) {
                container.remove(&menu.widget);
            }
        }
    };
}
