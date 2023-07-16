use crate::clients::system_tray::get_tray_event_client;
use crate::config::CommonConfig;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{await_sync, try_send};
use color_eyre::Result;
use gtk::gdk_pixbuf::{Colorspace, InterpType};
use gtk::prelude::*;
use gtk::{
    gdk_pixbuf, IconLookupFlags, IconTheme, Image, Label, Menu, MenuBar, MenuItem,
    SeparatorMenuItem,
};
use serde::Deserialize;
use std::collections::HashMap;
use stray::message::menu::{MenuItem as MenuItemInfo, MenuType};
use stray::message::tray::StatusNotifierItem;
use stray::message::{NotifierItemCommand, NotifierItemMessage};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Deserialize, Clone)]
pub struct TrayModule {
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

/// Attempts to get a GTK `Image` component
/// for the status notifier item's icon.
fn get_image_from_icon_name(item: &StatusNotifierItem) -> Option<Image> {
    let theme = item
        .icon_theme_path
        .as_ref()
        .map(|path| {
            let theme = IconTheme::new();
            theme.append_search_path(path);
            theme
        })
        .unwrap_or_default();

    item.icon_name.as_ref().and_then(|icon_name| {
        let icon_info = theme.lookup_icon(icon_name, 16, IconLookupFlags::empty());
        icon_info.map(|icon_info| Image::from_pixbuf(icon_info.load_icon().ok().as_ref()))
    })
}

/// Attempts to get an image from the item pixmap.
///
/// The pixmap is supplied in ARGB32 format,
/// which has 8 bits per sample and a bit stride of `4*width`.
fn get_image_from_pixmap(item: &StatusNotifierItem) -> Option<Image> {
    const BITS_PER_SAMPLE: i32 = 8; //

    let pixmap = item
        .icon_pixmap
        .as_ref()
        .and_then(|pixmap| pixmap.first())?;

    let bytes = glib::Bytes::from(&pixmap.pixels);
    let row_stride = pixmap.width * 4; //

    let pixbuf = gdk_pixbuf::Pixbuf::from_bytes(
        &bytes,
        Colorspace::Rgb,
        true,
        BITS_PER_SAMPLE,
        pixmap.width,
        pixmap.height,
        row_stride,
    );

    let pixbuf = pixbuf
        .scale_simple(16, 16, InterpType::Bilinear)
        .unwrap_or(pixbuf);
    Some(Image::from_pixbuf(Some(&pixbuf)))
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
                            try_send!(
                                tx,
                                NotifierItemCommand::MenuItemClicked {
                                    submenu_id: info.id,
                                    menu_path: path.clone(),
                                    notifier_address: id.clone(),
                                }
                            );
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

    fn name() -> &'static str {
        "tray"
    }

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
    ) -> Result<ModuleParts<MenuBar>> {
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
                        let addr = &address;
                        let menu_item = widgets.remove(address.as_str()).unwrap_or_else(|| {
                            let menu_item = MenuItem::new();
                            menu_item.style_context().add_class("item");

                            get_image_from_icon_name(&item)
                                .or_else(|| get_image_from_pixmap(&item))
                                .map_or_else(
                                    || {
                                        let label =
                                            Label::new(Some(item.title.as_ref().unwrap_or(addr)));
                                        menu_item.add(&label);
                                    },
                                    |image| {
                                        image.set_widget_name(address.as_str());
                                        menu_item.add(&image);
                                    },
                                );

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

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
