use crate::channels::AsyncSenderExt;
use crate::gtk_helpers::{IronbarGtkExt, MouseButton};
use glib::{Bytes, VariantTy};
use gtk::gdk::Texture;
use gtk::gio::{Icon, Menu, MenuModel, SimpleAction, SimpleActionGroup};
use gtk::{
    Box as GtkBox, Orientation, Shortcut, ShortcutAction, ShortcutController, ShortcutTrigger,
    prelude::*,
};
use gtk::{Button, Image, Label, PopoverMenu};
use system_tray::client::ActivateRequest;
use system_tray::item::{IconPixmap, Status, StatusNotifierItem, Tooltip};
use system_tray::menu::ToggleState;
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

/// Main tray icon to show on the bar
pub(crate) struct TrayMenu {
    pub box_content: GtkBox,
    pub widget: Button,
    pub popover: PopoverMenu,
    image_widget: Option<Image>,
    label_widget: Option<Label>,
    activated_channel: mpsc::Sender<ActivateRequest>,
    path: Option<String>,
    address: String,

    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_theme_path: Option<String>,
    pub icon_pixmap: Option<Vec<IconPixmap>>,
}

impl TrayMenu {
    pub fn new(
        address: &str,
        item: StatusNotifierItem,
        activated_channel: mpsc::Sender<ActivateRequest>,
    ) -> Self {
        let popover = PopoverMenu::builder().build(); // no `new` and we do not have a model yet
        let widget = Button::new();
        let content = GtkBox::new(Orientation::Horizontal, 0);

        let a = address.to_owned();
        let tx = activated_channel.clone();

        widget.connect_pressed(MouseButton::Primary, move || {
            trace!("pressed");
            let tx = tx.clone();
            let address = a.clone();

            tx.send_spawn(ActivateRequest::Default {
                address: address.clone(),
                x: 0,
                y: 0,
            });
        });

        let a = address.to_owned();
        let tx = activated_channel.clone();
        let pe = popover.clone();

        let has_menu = item.menu.is_some();

        widget.connect_pressed(MouseButton::Secondary, move || {
            trace!("secondary");
            pe.popup();

            if !has_menu {
                let tx = tx.clone();
                let address = a.clone();

                tx.send_spawn(ActivateRequest::Secondary {
                    address: address.clone(),
                    x: 0,
                    y: 0,
                });
            }
        });
        widget.set_child(Some(&content));
        widget.add_css_class("item");

        popover.set_parent(&widget);

        widget.set_visible(item.status != Status::Passive);

        if item.status == Status::NeedsAttention {
            widget.add_css_class("urgent");
        }

        Self {
            box_content: content,
            widget,
            popover,
            image_widget: None,
            label_widget: None,
            activated_channel,
            title: item.title,
            icon_name: item.icon_name,
            icon_theme_path: item.icon_theme_path,
            icon_pixmap: item.icon_pixmap,
            path: None,
            address: address.to_owned(),
        }
    }

    /// Updates the label text, and shows it in favour of the image.
    pub fn set_label(&mut self, text: &str) {
        if let Some(image) = self.image_widget.take() {
            image.set_visible(false);
        }

        self.label_widget
            .get_or_insert_with(|| {
                let label = Label::new(None);
                self.box_content.append(&label);
                label
            })
            .set_label(text);
    }

    /// Shows the label, using its current text.
    /// The image is hidden if present.
    pub fn show_label(&self) {
        if let Some(image) = &self.image_widget {
            image.set_visible(false);
        }

        if let Some(label) = &self.label_widget {
            label.set_visible(true);
        }
    }

    /// Updates the image, and shows it in favour of the label.
    pub fn set_image(&mut self, image: &Image) {
        if let Some(label) = self.label_widget.take() {
            label.set_visible(false);
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
        if let Some(icn) = &icon_name {
            self.widget.set_icon_name(icn);
        }
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

    pub fn set_status(&self, status: Status) {
        self.widget.set_visible(status != Status::Passive);

        if status == Status::NeedsAttention {
            self.widget.add_css_class("urgent");
        } else {
            self.widget.remove_css_class("urgent");
        }
    }

    pub fn set_menu(&mut self, menu: &str) {
        trace!("set menu {}", menu);
        self.path = Some(menu.to_owned());
    }

    pub fn set_menu_widget(&self, tray_menu: &system_tray::menu::TrayMenu) {
        debug!("set menu");

        let action_group = SimpleActionGroup::new();
        let shortcut_controller = ShortcutController::new();

        let model: MenuModel = self
            .as_menu(&tray_menu.submenus, &action_group, &shortcut_controller)
            .into();

        self.popover.set_menu_model(Some(&model));
        self.widget.insert_action_group("menu", Some(&action_group));
        self.widget.add_controller(shortcut_controller);
    }

    pub fn connect_item(
        &self,
        sub: &system_tray::menu::MenuItem,
        action_group: &SimpleActionGroup,
    ) -> String {
        let action_name = format!("action_{}", sub.id);
        let tx = self.activated_channel.clone();
        let id = sub.id;
        let action = SimpleAction::new(&action_name, None);
        let address = self.address.clone();

        if let Some(path) = self.path.clone() {
            action.connect_activate(move |_, _| activate(&tx, &address, &path, id));
        }

        action_group.add_action(&action);
        format!("menu.{action_name}")
    }

    pub fn connect_checkmark_item(
        &self,
        sub: &system_tray::menu::MenuItem,
        action_group: &SimpleActionGroup,
        value: bool,
    ) -> String {
        let action_name = format!("action_{}", sub.id);
        let tx = self.activated_channel.clone();
        let id = sub.id;
        let action = SimpleAction::new_stateful(&action_name, None, &value.to_variant());

        action.set_state(&value.to_variant());

        let address = self.address.clone();

        if let Some(path) = self.path.clone() {
            action.connect_change_state(move |_, _| activate(&tx, &address, &path, id));

            action.connect_change_state(move |ac, _| {
                let state = ac.state();

                if let Some(st) = state {
                    ac.set_state(&(!st.get::<bool>().unwrap_or(false)).to_variant());
                } else {
                    ac.set_state(&true.to_variant());
                }
            });
        }

        action_group.add_action(&action);
        format!("menu.{action_name}")
    }

    pub fn connect_radio_item(
        &self,
        sub: &system_tray::menu::MenuItem,
        action_group: &SimpleActionGroup,
        radio_group: &str,
        value: &str,
        selected: bool,
    ) -> String {
        let action_name = format!("action_radio_{radio_group}");
        let tx = self.activated_channel.clone();
        let id = sub.id;

        let action =
            SimpleAction::new_stateful(&action_name, Some(VariantTy::STRING), &value.to_variant());

        if selected {
            action.set_state(&value.to_variant());
        }

        let address = self.address.clone();

        if let Some(path) = self.path.clone() {
            action.connect_change_state(move |_, _| activate(&tx, &address, &path, id));
        }

        action_group.add_action(&action);
        format!("menu.{action_name}")
    }

    pub fn connect_shortcut(
        sub: &system_tray::menu::MenuItem,
        shortcut_controller: &ShortcutController,
    ) {
        if let Some(shortcuts) = &sub.shortcut {
            let shortcut = shortcuts
                .iter()
                .map(|e| e.join("+"))
                .collect::<Vec<_>>()
                .join("|");

            debug!("shortcut '{}' for menu id: {}", shortcut, sub.id);

            let shortcut = Shortcut::new(
                ShortcutTrigger::parse_string(&shortcut),
                ShortcutAction::parse_string("activate"),
            );

            shortcut_controller.add_shortcut(shortcut);
        }
    }

    fn as_menu(
        &self,
        items: &[system_tray::menu::MenuItem],
        action_group: &SimpleActionGroup,
        shortcut_controller: &ShortcutController,
    ) -> Menu {
        use gtk::gio::{MenuItem, MenuModel};
        use system_tray::menu::{MenuType, ToggleType};
        let mut section_container: Option<Menu> = None;

        // As current implementation it identifies radio groups based on the
        // item of type radio coming one after the other,
        // if there is a gap than a new radio group is started,
        // for handling multiple radio groups it use a sequential one of each group used as key for the action
        let mut radio_group_sequential = 0;
        let mut radio_group = None;
        let mut model = Menu::new();

        for sub in items {
            if !sub.visible {
                continue;
            }

            Self::connect_shortcut(sub, shortcut_controller);

            match sub.menu_type {
                MenuType::Standard => {
                    let label = sub.label.as_deref();
                    debug!("has children: '{:?}'", sub.children_display);

                    let item = if sub.children_display == Some("submenu".to_owned()) {
                        radio_group = None;
                        let submenu: MenuModel = self
                            .as_menu(&sub.submenu, action_group, shortcut_controller)
                            .into();

                        MenuItem::new_submenu(label, &submenu)
                    } else {
                        let action = if sub.enabled {
                            match sub.toggle_type {
                                ToggleType::Radio => {
                                    let value = match sub.toggle_state {
                                        ToggleState::On => true,
                                        ToggleState::Off | ToggleState::Indeterminate => false,
                                    };

                                    let target = format!("{}", sub.id);

                                    let rg = if let Some(rg) = radio_group {
                                        rg
                                    } else {
                                        radio_group_sequential += 1;

                                        let id = radio_group_sequential.to_string();

                                        self.connect_radio_item(
                                            sub,
                                            action_group,
                                            &id,
                                            &target,
                                            value,
                                        )
                                    };
                                    debug!("radio item {label:?}");

                                    radio_group = Some(rg.clone());
                                    format!("{rg}::{target}")
                                }
                                ToggleType::Checkmark => {
                                    radio_group = None;

                                    let value = match sub.toggle_state {
                                        ToggleState::On => true,
                                        ToggleState::Off | ToggleState::Indeterminate => false,
                                    };

                                    debug!("check item {:?} value {}", label, value);

                                    self.connect_checkmark_item(sub, action_group, value)
                                }
                                ToggleType::CannotBeToggled => {
                                    radio_group = None;
                                    debug!("item {:?}", label);
                                    self.connect_item(sub, action_group)
                                }
                            }
                        } else {
                            debug!("disabled item {:?}", label);
                            format!("action_{}", sub.id)
                        };

                        MenuItem::new(label, Some(action.as_str()))
                    };

                    debug!("inserting {}", sub.id);

                    // icons only show on MenuItems with no label in GTK4
                    // which is stupid given everything has a label
                    // but this logic remains just in case
                    if let Some(icon) = &sub.icon_name
                        && let Ok(ic) = Icon::for_string(icon)
                    {
                        item.set_icon(&ic);
                    } else if let Some(pixmap) = &sub.icon_data {
                        let bytes = Bytes::from(pixmap);
                        let texture = Texture::from_bytes(&bytes);

                        match texture {
                            Ok(texture) => {
                                item.set_icon(&Icon::from(texture));
                            }
                            Err(err) => {
                                error!("error loading texture: {err:?}");
                            }
                        }
                    }

                    model.insert_item(sub.id, &item);
                }

                MenuType::Separator => {
                    radio_group = None;
                    let label = sub.label.as_deref();

                    section_container = if let Some(section) = section_container {
                        section.insert_item(sub.id, &MenuItem::new_section(label, &model));
                        Some(section)
                    } else {
                        let sc = Menu::new();
                        sc.insert_item(sub.id, &MenuItem::new_section(label, &model));
                        Some(sc)
                    };

                    model = Menu::new();
                }
            }
        }

        if let Some(section) = section_container {
            section.insert_item(0, &MenuItem::new_section(None, &model));
            section
        } else {
            model
        }
    }
}

fn activate(tx: &mpsc::Sender<ActivateRequest>, address: &str, path: &str, id: i32) {
    trace!("activated {},{}, {}", address, path, id);
    let tx = tx.clone();
    let address = address.to_string();
    let path = path.to_string();

    tx.send_spawn(ActivateRequest::MenuItem {
        address,
        menu_path: path,
        submenu_id: id,
    });
}
