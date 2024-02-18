mod diff;
mod icon;
mod interface;

use crate::clients::system_tray::TrayEventReceiver;
use crate::config::CommonConfig;
use crate::modules::tray::diff::get_diffs;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, spawn};
use color_eyre::Result;
use gtk::{prelude::*, PackDirection};
use gtk::{IconTheme, MenuBar};
use interface::TrayMenu;
use serde::Deserialize;
use std::collections::HashMap;
use system_tray::message::{NotifierItemCommand, NotifierItemMessage};
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone)]
pub struct TrayModule {
    #[serde(default, deserialize_with = "deserialize_orientation")]
    pub direction: Option<PackDirection>,
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
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
    type SendMessage = NotifierItemMessage;
    type ReceiveMessage = NotifierItemCommand;

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

        let client = context.client::<TrayEventReceiver>();

        let (tray_tx, mut tray_rx) = client.subscribe();

        // listen to tray updates
        spawn(async move {
            while let Ok(message) = tray_rx.recv().await {
                tx.send(ModuleUpdateEvent::Update(message)).await?;
            }

            Ok::<(), mpsc::error::SendError<ModuleUpdateEvent<Self::SendMessage>>>(())
        });

        // send tray commands
        spawn(async move {
            while let Some(cmd) = rx.recv().await {
                tray_tx.send(cmd).await?;
            }

            Ok::<(), mpsc::error::SendError<NotifierItemCommand>>(())
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
                on_update(update, &container, &mut menus, &icon_theme, &context.controller_tx)
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
    update: NotifierItemMessage,
    container: &MenuBar,
    menus: &mut HashMap<Box<str>, TrayMenu>,
    icon_theme: &IconTheme,
    tx: &mpsc::Sender<NotifierItemCommand>,
) {
    match update {
        NotifierItemMessage::Update {
            item,
            address,
            menu,
        } => {
            if let (Some(menu_opts), Some(menu_path)) = (menu, &item.menu) {
                let submenus = menu_opts.submenus;

                let mut menu_item = menus.remove(address.as_str()).unwrap_or_else(|| {
                    let item = TrayMenu::new(tx.clone(), address.clone(), menu_path.to_string());
                    container.add(&item.widget);

                    item
                });

                let label = item.title.as_ref().unwrap_or(&address);
                if let Some(label_widget) = menu_item.label_widget() {
                    label_widget.set_label(label);
                }

                if item.icon_name.as_ref() != menu_item.icon_name() {
                    match icon::get_image_from_icon_name(&item, icon_theme)
                        .or_else(|| icon::get_image_from_pixmap(&item))
                    {
                        Some(image) => menu_item.set_image(&image),
                        None => menu_item.set_label(label),
                    };
                }

                let diffs = get_diffs(menu_item.state(), &submenus);
                menu_item.apply_diffs(diffs);
                menu_item.widget.show();

                menu_item.set_state(submenus);
                menu_item.set_icon_name(item.icon_name);

                menus.insert(address.into(), menu_item);
            }
        }
        NotifierItemMessage::Remove { address } => {
            if let Some(menu) = menus.get(address.as_str()) {
                container.remove(&menu.widget);
            }
        }
    };
}
