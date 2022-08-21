use crate::modules::{Module, ModuleInfo};
use color_eyre::Result;
use futures_util::StreamExt;
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme, Image, Menu, MenuBar, MenuItem, SeparatorMenuItem};
use serde::Deserialize;
use std::collections::HashMap;
use stray::message::menu::{MenuItem as MenuItemInfo, MenuType, TrayMenu};
use stray::message::tray::StatusNotifierItem;
use stray::message::{NotifierItemCommand, NotifierItemMessage};
use stray::SystemTray;
use tokio::spawn;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone)]
pub struct TrayModule;

#[derive(Debug)]
enum TrayUpdate {
    Update(String, Box<StatusNotifierItem>, Option<TrayMenu>),
    Remove(String),
}

/// Gets a GTK `Image` component
/// for the status notifier item's icon.
fn get_icon(item: &StatusNotifierItem) -> Option<Image> {
    item.icon_theme_path.as_ref().and_then(|path| {
        let theme = IconTheme::new();
        theme.append_search_path(&path);

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
    tx: &mpsc::Sender<NotifierItemCommand>,
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
    fn into_widget(self, _info: &ModuleInfo) -> Result<MenuBar> {
        let container = MenuBar::new();

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let (ui_tx, ui_rx) = mpsc::channel(32);

        spawn(async move {
            // FIXME: Can only spawn one of these at a time - means cannot have tray on multiple bars
            let mut tray = SystemTray::new(ui_rx).await;

            // listen for tray updates & send message to update UI
            while let Some(message) = tray.next().await {
                match message {
                    NotifierItemMessage::Update {
                        address: id,
                        item,
                        menu,
                    } => {
                        tx.send(TrayUpdate::Update(id, Box::new(item), menu))
                            .expect("Failed to send tray update event");
                    }
                    NotifierItemMessage::Remove { address: id } => {
                        tx.send(TrayUpdate::Remove(id))
                            .expect("Failed to send tray remove event");
                    }
                }
            }
        });

        {
            let container = container.clone();
            let mut widgets = HashMap::new();

            // listen for UI updates
            rx.attach(None, move |update| {
                match update {
                    TrayUpdate::Update(id, item, menu) => {
                        let menu_item = widgets.remove(id.as_str()).unwrap_or_else(|| {
                            let menu_item = MenuItem::new();
                            menu_item.style_context().add_class("item");
                            if let Some(image) = get_icon(&item) {
                                image.set_widget_name(id.as_str());
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
                                get_menu_items(&submenus, &ui_tx.clone(), &id, &menu_path)
                                    .iter()
                                    .for_each(|item| menu.add(item));
                                menu_item.set_submenu(Some(&menu));
                            }
                        }

                        widgets.insert(id, menu_item);
                    }
                    TrayUpdate::Remove(id) => {
                        if let Some(widget) = widgets.get(&id) {
                            container.remove(widget);
                        }
                    }
                };

                Continue(true)
            });
        };

        Ok(container)
    }
}
