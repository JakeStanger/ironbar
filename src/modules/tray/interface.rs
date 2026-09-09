use crate::channels::AsyncSenderExt;
use crate::gtk_helpers::{IronbarGtkExt, MouseButton};
use crate::modules::tray::{ReservedTrayAction, TrayClickAction, TrayClickHandlers, UiEvent};
use crate::script::Script;
use crate::spawn;
use glib::{Bytes, VariantTy};
use gtk::gdk::Texture;
use gtk::gio::{Icon, Menu, MenuModel, SimpleAction, SimpleActionGroup};
use gtk::{
    Box as GtkBox, Orientation, Picture, Shortcut, ShortcutAction, ShortcutController,
    ShortcutTrigger, prelude::*,
};
use gtk::{Button, Label, PopoverMenu};
use std::path::PathBuf;
use system_tray::client::ActivateRequest;
use system_tray::item::{IconPixmap, Status, StatusNotifierItem, Tooltip};
use system_tray::menu::{MenuDiff, ToggleState};
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

/// Main tray icon to show on the bar
#[derive(Debug)]
pub(crate) struct TrayMenu {
    pub box_content: GtkBox,
    pub widget: Button,
    pub popover: PopoverMenu,
    image_widget: Option<Picture>,
    label_widget: Option<Label>,
    tx: mpsc::Sender<UiEvent>,
    path: Option<String>,
    address: String,

    /// The last full menu model received, retained so incremental
    /// [`MenuDiff`]s (post-click property changes) have something to patch.
    menu: Option<system_tray::menu::TrayMenu>,

    /// The shortcut controller currently attached to the widget, retained so
    /// the next rebuild can detach it — `add_controller` appends, and nothing
    /// else ever removes.
    shortcut_controller: Option<ShortcutController>,

    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_theme_path: Option<PathBuf>,
    pub icon_pixmap: Option<Vec<IconPixmap>>,
}

impl TrayMenu {
    pub fn new(
        address: &str,
        item: StatusNotifierItem,
        tx: mpsc::Sender<UiEvent>,
        click_handlers: &TrayClickHandlers,
    ) -> Self {
        let popover = PopoverMenu::builder().build(); // no `new` and we do not have a model yet
        let widget = Button::new();
        let content = GtkBox::new(Orientation::Horizontal, 0);

        let has_menu = item.menu.is_some();

        // Capture metadata for placeholder substitution in custom commands
        let item_name = if !item.id.is_empty() {
            item.id.clone()
        } else {
            address.to_owned()
        };
        let item_title = item.title.clone();
        let item_icon_name = item.icon_name.clone();

        // Helper to execute a tray click action
        let execute_action = |action: TrayClickAction,
                              popover: &PopoverMenu,
                              tx: &mpsc::Sender<UiEvent>,
                              address: &str,
                              has_menu: bool,
                              name: &str,
                              title: &Option<String>,
                              icon_name: &Option<String>| {
            match action {
                TrayClickAction::Reserved(ReservedTrayAction::Menu) => {
                    trace!("TrayClickAction::Reserved(Menu)");

                    if has_menu {
                        popover.popup();
                        tx.send_spawn(UiEvent::Menu(true));
                    } else {
                        tx.send_spawn(UiEvent::Activate(ActivateRequest::Secondary {
                            address: address.to_owned(),
                            x: 0,
                            y: 0,
                        }));
                    }
                }
                TrayClickAction::Reserved(ReservedTrayAction::Default) => {
                    trace!("TrayClickAction::Reserved(Default)");
                    tx.send_spawn(UiEvent::Activate(ActivateRequest::Default {
                        address: address.to_owned(),
                        x: 0,
                        y: 0,
                    }));
                }
                TrayClickAction::Reserved(ReservedTrayAction::Secondary) => {
                    trace!("TrayClickAction::Reserved(Secondary)");
                    tx.send_spawn(UiEvent::Activate(ActivateRequest::Secondary {
                        address: address.to_owned(),
                        x: 0,
                        y: 0,
                    }));
                }
                TrayClickAction::Reserved(ReservedTrayAction::None) => {
                    trace!("TrayClickAction::Reserved(None) (ignoring)");
                }
                TrayClickAction::Custom(cmd) => {
                    trace!("TrayClickAction::Custom: {}", cmd);

                    // Substitute placeholders with tray item metadata
                    let cmd = cmd
                        .replace("{name}", name)
                        .replace("{title}", title.as_deref().unwrap_or(""))
                        .replace("{icon}", icon_name.as_deref().unwrap_or(""))
                        .replace("{address}", address);

                    trace!("Executing command after substitution: {}", cmd);
                    let script = Script::from(cmd.as_str());
                    spawn(async move {
                        if let Err(err) = script.get_output(None).await {
                            error!("{err:?}");
                        }
                    });
                }
            }
        };

        // Helper to create a click handler closure with all necessary context
        let make_handler = |action: &TrayClickAction| {
            let pe = popover.clone();
            let tx = tx.clone();
            let address_owned = address.to_owned();
            let action = action.clone();
            let name = item_name.clone();
            let title = item_title.clone();
            let icon = item_icon_name.clone();

            move || {
                execute_action(
                    action.clone(),
                    &pe,
                    &tx,
                    &address_owned,
                    has_menu,
                    &name,
                    &title,
                    &icon,
                );
            }
        };

        // Set up left-click handler with optional double-click support
        if click_handlers.on_click_left.is_actionable()
            || click_handlers.on_click_left_double.is_actionable()
        {
            let on_single = make_handler(&click_handlers.on_click_left);
            let on_double = if click_handlers.on_click_left_double.is_actionable() {
                Some(make_handler(&click_handlers.on_click_left_double))
            } else {
                None
            };

            widget.connect_pressed_with_double_click(MouseButton::Primary, on_single, on_double);
        }

        // Set up right-click handler with optional double-click support
        if click_handlers.on_click_right.is_actionable()
            || click_handlers.on_click_right_double.is_actionable()
        {
            let on_single = make_handler(&click_handlers.on_click_right);
            let on_double = if click_handlers.on_click_right_double.is_actionable() {
                Some(make_handler(&click_handlers.on_click_right_double))
            } else {
                None
            };

            widget.connect_pressed_with_double_click(MouseButton::Secondary, on_single, on_double);
        }

        // Set up middle-click handler with optional double-click support
        if click_handlers.on_click_middle.is_actionable()
            || click_handlers.on_click_middle_double.is_actionable()
        {
            let on_single = make_handler(&click_handlers.on_click_middle);
            let on_double = if click_handlers.on_click_middle_double.is_actionable() {
                Some(make_handler(&click_handlers.on_click_middle_double))
            } else {
                None
            };

            widget.connect_pressed_with_double_click(MouseButton::Middle, on_single, on_double);
        }

        widget.set_child(Some(&content));
        widget.add_css_class("item");

        popover.set_parent(&widget);

        widget.set_visible(item.status != Status::Passive);

        if item.status == Status::NeedsAttention {
            widget.add_css_class("urgent");
        }

        popover.connect_hide({
            let tx = tx.clone();
            move |_| tx.send_spawn(UiEvent::Menu(false))
        });

        Self {
            box_content: content,
            widget,
            popover,
            image_widget: None,
            label_widget: None,
            tx,
            title: item.title,
            icon_name: item.icon_name,
            icon_theme_path: item.icon_theme_path.map(PathBuf::from),
            icon_pixmap: item.icon_pixmap,
            path: None,
            address: address.to_owned(),
            menu: None,
            shortcut_controller: None,
        }
    }

    /// Updates the label text, and shows it in favour of the image.
    pub fn set_label(&mut self, text: &str) {
        if let Some(image) = &self.image_widget {
            image.set_visible(false);
        }

        let label = self.label_widget.get_or_insert_with(|| {
            let label = Label::new(None);
            self.box_content.append(&label);
            label
        });

        label.set_label(text);
        label.set_visible(true);
    }

    /// Updates the image, and shows it in favour of the label.
    pub fn set_image(&mut self, image: &Picture) {
        let tooltip = self.widget.tooltip_text();

        if let Some(label) = &self.label_widget {
            label.set_visible(false);
        }

        if let Some(old) = self.image_widget.replace(image.clone()) {
            self.box_content.remove(&old);
        }

        self.box_content.append(image);
        image.set_tooltip_text(tooltip.as_deref());
    }

    pub fn image_widget(&self) -> Option<&Picture> {
        self.image_widget.as_ref()
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
        let title = title.as_deref();

        self.widget.set_tooltip_text(title);

        if let Some(widget) = &self.image_widget {
            widget.set_tooltip_text(title);
        }

        if let Some(widget) = &self.label_widget {
            widget.set_tooltip_text(title);
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

    /// Replaces the menu with a freshly received full model and repaints.
    pub fn set_menu_widget(&mut self, tray_menu: &system_tray::menu::TrayMenu) {
        self.menu = Some(tray_menu.clone());
        self.build_menu_widget();
    }

    /// Applies incremental dbusmenu property updates to the retained model and
    /// repaints, so post-click changes (a moved radio selection, a new label)
    /// become visible without the caller re-sending the whole menu.
    pub fn apply_menu_diff(&mut self, diffs: &[MenuDiff]) {
        match self.menu.as_mut() {
            Some(menu) => super::diff::apply_menu_diffs(&mut menu.submenus, diffs),
            None => {
                // A diff arrived before any full menu — nothing to patch.
                trace!("received menu diff with no menu to apply it to");
                return;
            }
        }

        self.build_menu_widget();
    }

    /// Rebuilds the popover's menu model, action group, and shortcuts from the
    /// retained menu model.
    fn build_menu_widget(&mut self) {
        let Some(tray_menu) = self.menu.as_ref() else {
            return;
        };

        debug!("set menu");

        let action_group = SimpleActionGroup::new();
        let shortcut_controller = ShortcutController::new();

        let model: MenuModel = self
            .as_menu(&tray_menu.submenus, &action_group, &shortcut_controller)
            .into();

        self.popover.set_menu_model(Some(&model));
        self.widget.insert_action_group("menu", Some(&action_group));

        // `insert_action_group` replaces the previous group by name, but
        // `add_controller` only appends — detach the previous controller or
        // they accumulate on the widget for the lifetime of the tray item.
        if let Some(old) = self
            .shortcut_controller
            .replace(shortcut_controller.clone())
        {
            self.widget.remove_controller(&old);
        }
        self.widget.add_controller(shortcut_controller);
    }

    pub fn connect_item(
        &self,
        sub: &system_tray::menu::MenuItem,
        action_group: &SimpleActionGroup,
    ) -> String {
        let action_name = format!("action_{}", sub.id);
        let tx = self.tx.clone();
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
        let tx = self.tx.clone();
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

    /// Registers the single stateful action shared by every item of one radio
    /// group, returning it alongside its `menu.`-qualified name.
    ///
    /// The caller points each item at `"{name}::{item target}"` and parks the
    /// state on the selected item; the action itself cannot tell the options
    /// apart, so the target is the only thing identifying which one was
    /// clicked.
    fn connect_radio_group(
        &self,
        action_group: &SimpleActionGroup,
        group_id: i32,
        initial_target: &str,
    ) -> (SimpleAction, String) {
        let action_name = radio_action_name(group_id);
        let tx = self.tx.clone();
        let address = self.address.clone();

        let action = SimpleAction::new_stateful(
            &action_name,
            Some(VariantTy::STRING),
            &initial_target.to_variant(),
        );

        if let Some(path) = self.path.clone() {
            action.connect_change_state(move |action, state| {
                let Some(state) = state else { return };

                let Some(id) = radio_target_id(state) else {
                    error!("radio action state is not a menu item id: {state:?}");
                    return;
                };

                // GTK does not advance a stateful action's state for us on
                // change-state, so without this the mark never moves.
                action.set_state(state);

                activate(&tx, &address, &path, id);
            });
        }

        action_group.add_action(&action);
        (action, format!("menu.{action_name}"))
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

        // dbusmenu has no explicit radio grouping, so a group is a run of
        // consecutive radio items; anything else — another toggle type, or a
        // submenu — ends the run. The run's shared action is created on its
        // first item and reused by the rest.
        let mut radio_group: Option<(SimpleAction, String)> = None;
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
                                    let target = format!("{}", sub.id);

                                    let (group_action, action_name) = radio_group
                                        .get_or_insert_with(|| {
                                            self.connect_radio_group(action_group, sub.id, &target)
                                        });

                                    // Park the group's state on the item the
                                    // application reports as selected, so the
                                    // mark lands there rather than on whichever
                                    // item happened to create the action.
                                    if matches!(sub.toggle_state, ToggleState::On) {
                                        group_action.set_state(&target.to_variant());
                                    }

                                    debug!("radio item {label:?}");

                                    format!("{action_name}::{target}")
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

/// Names the GTK action shared by one radio group.
///
/// Every radio group in a tray menu — including groups nested in different
/// submenus — is registered into the *same* `SimpleActionGroup`, and
/// `add_action` silently replaces an existing action of the same name. Deriving
/// the name from the group's first dbusmenu id, which is unique across the whole
/// menu, makes that collision impossible by construction.
fn radio_action_name(group_id: i32) -> String {
    format!("action_radio_{group_id}")
}

/// Reads the clicked item's dbusmenu id out of a radio action's new state.
///
/// One action backs a whole radio group, so this target is the only thing
/// distinguishing the options; dropping it would send every click to whichever
/// item created the action.
fn radio_target_id(state: &glib::Variant) -> Option<i32> {
    state.str()?.parse().ok()
}

fn activate(tx: &mpsc::Sender<UiEvent>, address: &str, path: &str, id: i32) {
    trace!("activated {},{}, {}", address, path, id);
    let tx = tx.clone();
    let address = address.to_string();
    let path = path.to_string();

    tx.send_spawn(UiEvent::Activate(ActivateRequest::MenuItem {
        address,
        menu_path: path,
        submenu_id: id,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Radio groups from different submenus all land in one
    /// `SimpleActionGroup`, where `add_action` replaces on a duplicate name —
    /// so two groups sharing a name means only one survives and every radio
    /// item in the menu drives it. Distinct first-item ids are what prevent
    /// that.
    #[test]
    fn groups_with_different_first_items_get_different_action_names() {
        assert_ne!(radio_action_name(5), radio_action_name(19));
    }

    /// `as_menu` targets each radio item with `format!("{}", sub.id)`; this is
    /// the other half of that contract. If the two formats ever drift apart,
    /// clicks stop resolving and the menu silently does nothing.
    #[test]
    fn target_round_trips_the_menu_item_id() {
        for id in [0, 1, 19, i32::MAX] {
            let target = format!("{id}");
            assert_eq!(radio_target_id(&target.to_variant()), Some(id));
        }
    }

    #[test]
    fn returns_none_when_state_is_not_a_menu_item_id() {
        assert_eq!(radio_target_id(&"not-an-id".to_variant()), None);
        assert_eq!(radio_target_id(&true.to_variant()), None);
    }
}
