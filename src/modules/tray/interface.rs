use glib::{Bytes, Propagation, SignalHandlerId};
use gtk::gdk::Gravity;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream, Menu, MenuItem};
use gtk::{Box as GtkBox, Orientation, prelude::*};
use gtk::{EventController, Image, Label, MenuButton, PopoverMenu};
use system_tray::item::{IconPixmap, StatusNotifierItem, Tooltip};
use tracing::{debug, error, trace, warn};

/// Main tray icon to show on the bar
pub(crate) struct TrayMenu {
    button_handler: Option<SignalHandlerId>,
    pub box_content: GtkBox,
    pub widget: MenuButton,
    pub popover: PopoverMenu,
    image_widget: Option<Image>,
    label_widget: Option<Label>,

    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_theme_path: Option<String>,
    pub icon_pixmap: Option<Vec<IconPixmap>>,
}

impl TrayMenu {
    pub fn new(address: &str, item: StatusNotifierItem) -> Self {
        let popover = PopoverMenu::builder().build();
        let widget = MenuButton::builder().build();
        let content = GtkBox::new(Orientation::Horizontal, 0);
        widget.set_popover(Some(&popover));
        widget.set_child(Some(&content));
        widget.style_context().add_class("item");

        let mut slf = Self {
            button_handler: None,
            box_content: content,
            widget,
            popover,
            image_widget: None,
            label_widget: None,
            title: item.title,
            icon_name: item.icon_name,
            icon_theme_path: item.icon_theme_path,
            icon_pixmap: item.icon_pixmap,
        };

        slf
    }

    /// Updates the label text, and shows it in favour of the image.
    pub fn set_label(&mut self, text: &str) {
        if let Some(image) = &self.image_widget {
            image.hide();
        }
        self.label_widget
            .get_or_insert_with(|| {
                let label = Label::new(None);
                self.box_content.append(&label);
                label.show();
                label
            })
            .set_label(text);
        self.widget.set_label(text);
    }

    /// Shows the label, using its current text.
    /// The image is hidden if present.
    pub fn show_label(&self) {
        if let Some(image) = &self.image_widget {
            image.hide();
        }

        if let Some(label) = &self.label_widget {
            label.show();
        }
    }

    /// Updates the image, and shows it in favour of the label.
    pub fn set_image(&mut self, image: &Image) {
        if let Some(label) = &self.label_widget {
            label.hide();
        }

        if let Some(old) = self.image_widget.replace(image.clone()) {
            self.box_content.remove(&old);
        }

        self.box_content.append(image);
    }

    pub fn label_widget(&self) -> Option<&Label> {
        self.label_widget.as_ref()
    }

    pub fn icon_name(&self) -> Option<&String> {
        self.icon_name.as_ref()
    }

    pub fn set_icon_name(&mut self, icon_name: Option<String>) {
        self.icon_name = icon_name;
    }

    pub fn set_tooltip(&self, tooltip: Option<Tooltip>) {
        let title = tooltip.map(|t| t.title);

        if let Some(widget) = &self.image_widget {
            widget.set_tooltip_text(title.as_deref());
        }

        if let Some(widget) = &self.label_widget {
            widget.set_tooltip_text(title.as_deref());
        }
    }

    pub fn set_menu_widget(&mut self, tray_menu: &system_tray::menu::TrayMenu) {
        debug!("set menu");
        use gtk::gio::MenuModel;
        let model: MenuModel = to_menu(&tray_menu.submenus).into();
        self.widget.set_menu_model(Some(&model));
    }
}

fn to_menu(items: &Vec<system_tray::menu::MenuItem>) -> Menu {
    use gtk::gio::{MenuItem, MenuModel};
    use system_tray::menu::{MenuType, ToggleType};

    let val = Menu::new();
    for sub in items.iter() {
        match sub.menu_type {
            MenuType::Standard => {
                let label = sub.label.as_ref().map(String::as_str);
                debug!("has children: '{:?}'", sub.children_display);
                let item = if sub.children_display == Some("submenu".to_owned()) {
                    let submenu: MenuModel = to_menu(&sub.submenu).into();
                    MenuItem::new_submenu(label, &submenu)
                } else {
                    //menu.m
                    match sub.toggle_type {
                        ToggleType::Radio => {
                            // TOOD: hadle the flag with the action
                            MenuItem::new(label, None)
                        }
                        ToggleType::Checkmark => {
                            // TOOD: hadle the flag with the action
                            MenuItem::new(label, None)
                        }
                        ToggleType::CannotBeToggled => {
                            debug!("new item {:?}", label);
                            MenuItem::new(label, None)
                        }
                    }
                };
                debug!("inserting {}", sub.id);
                val.insert_item(sub.id, &item);
            }
            MenuType::Separator => {}
        }
    }
    val
}
