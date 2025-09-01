use crate::spawn;
use glib::{Bytes, Propagation, SignalHandlerId, Variant, VariantTy};
use gtk::gdk::Gravity;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{
    Action, ActionEntry, Cancellable, Icon, MemoryInputStream, Menu, MenuItem, SimpleAction,
    SimpleActionGroup,
};
use gtk::{
    Box as GtkBox, Orientation, Shortcut, ShortcutAction, ShortcutController, ShortcutTrigger,
    prelude::*,
};
use gtk::{Button, EventController, Image, Label, MenuButton, PopoverMenu};
use system_tray::client::ActivateRequest;
use system_tray::item::{IconPixmap, StatusNotifierItem, Tooltip};
use system_tray::menu::ToggleState;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Main tray icon to show on the bar
pub(crate) struct TrayMenu {
    button_handler: Option<SignalHandlerId>,
    pub box_content: GtkBox,
    pub widget: Button,
    pub popover: PopoverMenu,
    image_widget: Option<Image>,
    label_widget: Option<Label>,
    action_group: Option<SimpleActionGroup>,
    shortcut: Option<ShortcutController>,
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
        let popover = PopoverMenu::builder().build();
        let widget = Button::builder().build();
        let content = GtkBox::new(Orientation::Horizontal, 0);
        let pe = popover.clone();
        let a = address.to_owned();
        let channel = activated_channel.clone();
        widget.connect_clicked(move |e| {
            trace!("pressed");
            let c = channel.clone();
            let a = a.clone();
            spawn(async move {
                c.send(ActivateRequest::Default {
                    address: a.to_owned(),
                    x: 0,
                    y: 0,
                })
                .await;
            });
        });
        let gesture = gtk::GestureClick::new();

        gesture.set_button(gtk::gdk::ffi::GDK_BUTTON_SECONDARY as u32);

        let a = address.to_owned();
        let channel = activated_channel.clone();
        gesture.connect_pressed(move |gesture, _, _, _| {
            trace!("secondary");
            pe.popup();
            gesture.set_state(gtk::EventSequenceState::Claimed);

            let c = channel.clone();
            let a = a.clone();
            spawn(async move {
                c.send(ActivateRequest::Secondary {
                    address: a.to_owned(),
                    x: 0,
                    y: 0,
                })
                .await;
            });
        });
        widget.add_controller(gesture);
        popover.set_parent(&widget);
        //widget.set_popover(Some(&popover));
        widget.set_child(Some(&content));
        widget.style_context().add_class("item");

        if let Some(pix) = &item.icon_pixmap {}

        let menu_path = item.menu.unwrap_or_else(|| "".to_owned());
        let mut slf = Self {
            button_handler: None,
            box_content: content,
            widget,
            popover,
            image_widget: None,
            label_widget: None,
            action_group: None,
            shortcut: None,
            activated_channel,
            title: item.title,
            icon_name: item.icon_name,
            icon_theme_path: item.icon_theme_path,
            icon_pixmap: item.icon_pixmap,
            path: None,
            address: address.to_owned(),
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
    pub fn set_menu(&mut self, menu: &str) {
        trace!("set menu {}", menu);
        self.path = Some(menu.to_owned());
    }

    pub fn set_menu_widget(&mut self, tray_menu: &system_tray::menu::TrayMenu) {
        debug!("set menu");
        use gtk::gio::MenuModel;
        let mut action_group = SimpleActionGroup::new();
        let mut shortcut_controller = ShortcutController::new();
        let model: MenuModel = self
            .to_menu(
                &tray_menu.submenus,
                &mut action_group,
                &mut shortcut_controller,
            )
            .into();
        self.popover.set_menu_model(Some(&model));
        self.widget.insert_action_group("base", Some(&action_group));
        self.widget.add_controller(shortcut_controller);
    }

    pub fn connect_item(
        &mut self,
        sub: &system_tray::menu::MenuItem,
        action_group: &mut SimpleActionGroup,
    ) -> String {
        let action_name = format!("action_{}", sub.id);
        let channel = self.activated_channel.clone();
        let id = sub.id;
        let lab = sub.label.clone();
        let action = SimpleAction::new(&action_name, None);
        let address = self.address.clone();
        if let Some(path) = self.path.clone() {
            action.connect_activate(move |_, _| {
                trace!("activated {},{}, {} {:?} ", address, path, id, lab);
                let c = channel.clone();
                let a = address.clone();
                let p = path.clone();
                spawn(async move {
                    c.send(ActivateRequest::MenuItem {
                        address: a,
                        menu_path: p,
                        submenu_id: id,
                    })
                    .await;
                });
            });
        } else {
            warn!("Cannoct connect menu action missing dbus path");
        }
        action_group.add_action(&action);
        format!("base.{}", &action_name)
    }

    pub fn connect_checkmark_item(
        &mut self,
        sub: &system_tray::menu::MenuItem,
        action_group: &mut SimpleActionGroup,
        value: bool,
    ) -> String {
        let action_name = format!("action_{}", sub.id);
        let channel = self.activated_channel.clone();
        let id = sub.id;
        let lab = sub.label.clone();
        let action = SimpleAction::new_stateful(&action_name, None, &value.to_variant());
        action.set_state(&value.to_variant());
        let address = self.address.clone();
        if let Some(path) = self.path.clone() {
            action.connect_change_state(move |ac, _| {
                trace!("activated {},{}, {} {:?} ", address, path, id, lab);
                let c = channel.clone();
                let a = address.clone();
                let p = path.clone();
                spawn(async move {
                    c.send(ActivateRequest::MenuItem {
                        address: a,
                        menu_path: p,
                        submenu_id: id,
                    })
                    .await;
                });
            });
            action.connect_change_state(move |ac, _| {
                let state = ac.state();
                if let Some(st) = state {
                    ac.set_state(&(!st.get::<bool>().unwrap_or(false)).to_variant());
                } else {
                    ac.set_state(&true.to_variant());
                }
            });
        } else {
            warn!("Cannoct connect menu action missing dbus path");
        }
        action_group.add_action(&action);
        format!("base.{}", &action_name)
    }

    pub fn connect_radio_item(
        &mut self,
        sub: &system_tray::menu::MenuItem,
        action_group: &mut SimpleActionGroup,
        radio_group: &str,
        value: &str,
        selected: bool,
    ) -> String {
        let action_name = format!("action_radio_{}", radio_group);
        let channel = self.activated_channel.clone();
        let id = sub.id;
        let lab = sub.label.clone();
        let action =
            SimpleAction::new_stateful(&action_name, Some(VariantTy::STRING), &value.to_variant());
        if selected {
            action.set_state(&value.to_variant());
        }
        let address = self.address.clone();
        if let Some(path) = self.path.clone() {
            action.connect_change_state(move |ac, _| {
                trace!("activated {},{}, {} {:?} ", address, path, id, lab);
                let c = channel.clone();
                let a = address.clone();
                let p = path.clone();
                spawn(async move {
                    c.send(ActivateRequest::MenuItem {
                        address: a,
                        menu_path: p,
                        submenu_id: id,
                    })
                    .await;
                });
            });
        } else {
            warn!("Cannoct connect menu action missing dbus path");
        }
        action_group.add_action(&action);
        format!("base.{}", &action_name)
    }
    pub fn connect_shortcut(
        &mut self,
        sub: &system_tray::menu::MenuItem,
        shortcut_controller: &mut ShortcutController,
    ) {
        if let Some(scs) = &sub.shortcut {
            // TODO: test this formatting for parsing
            let sc = scs
                .iter()
                .map(|e| e.join("+"))
                .collect::<Vec<_>>()
                .join("|");
            debug!("shourtcut '{}' for menu id: {}", sc, sub.id);
            let shortcut = Shortcut::new(
                ShortcutTrigger::parse_string(&sc),
                ShortcutAction::parse_string("activate"),
            );
            shortcut_controller.add_shortcut(shortcut);
        }
    }

    fn to_menu(
        &mut self,
        items: &Vec<system_tray::menu::MenuItem>,
        action_group: &mut SimpleActionGroup,
        shortcut_controller: &mut ShortcutController,
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
        let mut val = Menu::new();
        for sub in items.iter() {
            if !sub.visible {
                continue;
            }
            self.connect_shortcut(sub, shortcut_controller);
            match sub.menu_type {
                MenuType::Standard => {
                    let label = sub.label.as_ref().map(String::as_str);
                    debug!("has children: '{:?}'", sub.children_display);
                    let item = if sub.children_display == Some("submenu".to_owned()) {
                        radio_group = None;
                        let submenu: MenuModel = self
                            .to_menu(&sub.submenu, action_group, shortcut_controller)
                            .into();
                        MenuItem::new_submenu(label, &submenu)
                    } else {
                        let action = if sub.enabled {
                            match sub.toggle_type {
                                ToggleType::Radio => {
                                    let value = match sub.toggle_state {
                                        ToggleState::On => true,
                                        ToggleState::Off => false,
                                        ToggleState::Indeterminate => false,
                                    };
                                    let target = format!("{}", sub.id);
                                    let rg = if let Some(rg) = radio_group {
                                        rg
                                    } else {
                                        radio_group_sequential += 1;
                                        let id = format!("{}", radio_group_sequential);
                                        self.connect_radio_item(
                                            sub,
                                            action_group,
                                            &id,
                                            &target,
                                            value,
                                        )
                                    };
                                    debug!("radio item {:?}", label);
                                    radio_group = Some(rg.clone());
                                    format!("{}::{}", rg, target)
                                }
                                ToggleType::Checkmark => {
                                    radio_group = None;
                                    let value = match sub.toggle_state {
                                        ToggleState::On => true,
                                        ToggleState::Off => false,
                                        ToggleState::Indeterminate => false,
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
                    if let Some(icon) = &sub.icon_name {
                        if let Ok(ic) = Icon::for_string(icon) {
                            item.set_icon(&ic);
                        }
                    }
                    val.insert_item(sub.id, &item);
                }
                MenuType::Separator => {
                    radio_group = None;
                    let label = sub.label.as_ref().map(String::as_str);
                    section_container = if let Some(sc) = section_container {
                        sc.insert_item(sub.id, &MenuItem::new_section(label, &val));
                        Some(sc)
                    } else {
                        let sc = Menu::new();
                        sc.insert_item(sub.id, &MenuItem::new_section(label, &val));
                        Some(sc)
                    };
                    val = Menu::new();
                }
            }
        }

        if let Some(sc) = section_container {
            sc.insert_item(0, &MenuItem::new_section(None, &val));
            sc
        } else {
            val
        }
    }
}
