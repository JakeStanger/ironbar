mod client;

use crate::await_sync;
use crate::modules::tray::client::get_tray_event_client;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme, Image, Menu, MenuBar, MenuItem, SeparatorMenuItem};
use serde::Deserialize;
use std::collections::HashMap;
use stray::message::menu::{MenuItem as MenuItemInfo, MenuType};
use stray::message::tray::StatusNotifierItem;
use stray::message::{NotifierItemCommand, NotifierItemMessage};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Deserialize, Clone)]
pub struct TrayModule;

/// Gets a GTK `Image` component
/// for the status notifier item's icon.
fn get_icon(item: &StatusNotifierItem) -> Option<Image> {
    item.icon_theme_path.as_ref().and_then(|path| {
        let theme = IconTheme::new();
        theme.append_search_path(path);

        item.icon_name.as_ref().and_then(|icon_name| {
            let icon_info = theme.lookup_icon(icon_name, 16, IconLookupFlags::empty());
            icon_info.map(|icon_info| Image::from_pixbuf(icon_info.load_icon().ok().as_ref()))
        })
    })
}

/// Recursively gets GTK `MenuItem` components
/// for the provided submenu array.
fn get_menu_items(
    menu: &[MenuItemInfo],
    tx: &Sender<NotifierItemCommand>,
    id: &str,
    path: &str,
) -> Vec<MenuItem> {
    menu.iter()
        .map(|item_info| {
            let item: Box<dyn AsRef<MenuItem>> = match item_info.menu_type {
                MenuType::Separator => Box::new(SeparatorMenuItem::new()),
                MenuType::Standard => {
                    let mut builder = MenuItem::builder()
                        .label(item_info.label.as_str())
                        .visible(item_info.visible)
                        .sensitive(item_info.enabled);

                    if !item_info.submenu.is_empty() {
                        let menu = Menu::new();
                        get_menu_items(&item_info.submenu, &tx.clone(), id, path)
                            .iter()
                            .for_each(|item| menu.add(item));

                        builder = builder.submenu(&menu);
                    }

                    let item = builder.build();

                    let info = item_info.clone();
                    let id = id.to_string();
                    let path = path.to_string();

                    {
                        let tx = tx.clone();
                        item.connect_activate(move |_item| {
                            tx.try_send(NotifierItemCommand::MenuItemClicked {
                                submenu_id: info.id,
                                menu_path: path.clone(),
                                notifier_address: id.clone(),
                            })
                            .expect("Failed to send menu item clicked event");
                        });
                    }

                    Box::new(item)
                }
            };

            (*item).as_ref().clone()
        })
        .collect()
}

impl Module<MenuBar> for TrayModule {
    type SendMessage = NotifierItemMessage;
    type ReceiveMessage = NotifierItemCommand;

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let client = await_sync(async { get_tray_event_client().await });
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
        _info: &ModuleInfo,
    ) -> Result<ModuleWidget<MenuBar>> {
        let container = MenuBar::new();

        {
            let container = container.clone();
            let mut widgets = HashMap::new();

            // listen for UI updates
            context.widget_rx.attach(None, move |update| {
                match update {
                    NotifierItemMessage::Update {
                        item,
                        address,
                        menu,
                    } => {
                        let menu_item = widgets.remove(address.as_str()).unwrap_or_else(|| {
                            let menu_item = MenuItem::new();
                            menu_item.style_context().add_class("item");
                            if let Some(image) = get_icon(&item) {
                                image.set_widget_name(address.as_str());
                                menu_item.add(&image);
                            }
                            container.add(&menu_item);
                            menu_item.show_all();
                            menu_item
                        });
                        if let (Some(menu_opts), Some(menu_path)) = (menu, item.menu) {
                            let submenus = menu_opts.submenus;
                            if !submenus.is_empty() {
                                let menu = Menu::new();
                                get_menu_items(
                                    &submenus,
                                    &context.controller_tx.clone(),
                                    &address,
                                    &menu_path,
                                )
                                .iter()
                                .for_each(|item| menu.add(item));
                                menu_item.set_submenu(Some(&menu));
                            }
                        }
                        widgets.insert(address, menu_item);
                    }
                    NotifierItemMessage::Remove { address } => {
                        if let Some(widget) = widgets.get(&address) {
                            container.remove(widget);
                        }
                    }
                };

                Continue(true)
            });
        };

        Ok(ModuleWidget {
            widget: container,
            popup: None,
        })
    }
}
