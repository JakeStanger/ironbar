use crate::modules::tray::diff::{Diff, MenuItemDiff};
use crate::{spawn, try_send};
use gtk::prelude::*;
use gtk::{CheckMenuItem, Image, Label, Menu, MenuItem, SeparatorMenuItem};
use std::collections::HashMap;
use system_tray::message::menu::{MenuItem as MenuItemInfo, MenuType, ToggleState, ToggleType};
use system_tray::message::NotifierItemCommand;
use tokio::sync::mpsc;

/// Calls a method on the underlying widget,
/// passing in a single argument.
///
/// This is useful to avoid matching on
/// `TrayMenuWidget` constantly.
///
/// # Example
/// ```rust
/// call!(container, add, my_widget)
/// ```
/// is the same as:
/// ```
/// match &my_widget {
///     TrayMenuWidget::Separator(w) => {
///         container.add(w);
///     }
///     TrayMenuWidget::Standard(w) => {
///         container.add(w);
///     }
///     TrayMenuWidget::Checkbox(w) => {
///         container.add(w);
///     }
/// }
/// ```
macro_rules! call {
    ($parent:expr, $method:ident, $child:expr) => {
        match &$child {
            TrayMenuWidget::Separator(w) => {
                $parent.$method(w);
            }
            TrayMenuWidget::Standard(w) => {
                $parent.$method(w);
            }
            TrayMenuWidget::Checkbox(w) => {
                $parent.$method(w);
            }
        }
    };
}

/// Main tray icon to show on the bar
pub(crate) struct TrayMenu {
    pub(crate) widget: MenuItem,
    menu_widget: Menu,
    image_widget: Option<Image>,
    label_widget: Option<Label>,

    menu: HashMap<i32, TrayMenuItem>,
    state: Vec<MenuItemInfo>,
    icon_name: Option<String>,

    tx: mpsc::Sender<i32>,
}

impl TrayMenu {
    pub fn new(tx: mpsc::Sender<NotifierItemCommand>, address: String, path: String) -> Self {
        let widget = MenuItem::new();
        widget.style_context().add_class("item");

        let (item_tx, mut item_rx) = mpsc::channel(8);

        spawn(async move {
            while let Some(id) = item_rx.recv().await {
                try_send!(
                    tx,
                    NotifierItemCommand::MenuItemClicked {
                        submenu_id: id,
                        menu_path: path.clone(),
                        notifier_address: address.clone(),
                    }
                );
            }
        });

        let menu = Menu::new();
        widget.set_submenu(Some(&menu));

        Self {
            widget,
            menu_widget: menu,
            image_widget: None,
            label_widget: None,
            state: vec![],
            icon_name: None,
            menu: HashMap::new(),
            tx: item_tx,
        }
    }

    /// Updates the label text, and shows it in favour of the image.
    pub fn set_label(&mut self, text: &str) {
        if let Some(image) = &self.image_widget {
            image.hide();
        }

        self.label_widget
            .get_or_insert_with(|| {
                let label = Label::new(None);
                self.widget.add(&label);
                label.show();
                label
            })
            .set_label(text);
    }

    /// Updates the image, and shows it in favour of the label.
    pub fn set_image(&mut self, image: &Image) {
        if let Some(label) = &self.label_widget {
            label.hide();
        }

        if let Some(old) = self.image_widget.replace(image.clone()) {
            self.widget.remove(&old);
        }

        self.widget.add(image);
        image.show();
    }

    /// Applies a diff set to the submenu.
    pub fn apply_diffs(&mut self, diffs: Vec<Diff>) {
        for diff in diffs {
            match diff {
                Diff::Add(info) => {
                    let item = TrayMenuItem::new(&info, self.tx.clone());
                    call!(self.menu_widget, add, item.widget);
                    self.menu.insert(item.id, item);
                }
                Diff::Update(id, info) => {
                    if let Some(item) = self.menu.get_mut(&id) {
                        item.apply_diff(info);
                    }
                }
                Diff::Remove(id) => {
                    if let Some(item) = self.menu.remove(&id) {
                        call!(self.menu_widget, remove, item.widget);
                    }
                }
            }
        }
    }

    pub fn label_widget(&self) -> Option<&Label> {
        self.label_widget.as_ref()
    }

    pub fn state(&self) -> &[MenuItemInfo] {
        &self.state
    }

    pub fn set_state(&mut self, state: Vec<MenuItemInfo>) {
        self.state = state;
    }

    pub fn icon_name(&self) -> Option<&String> {
        self.icon_name.as_ref()
    }

    pub fn set_icon_name(&mut self, icon_name: Option<String>) {
        self.icon_name = icon_name;
    }
}

#[derive(Debug)]
struct TrayMenuItem {
    id: i32,
    widget: TrayMenuWidget,
    menu_widget: Menu,
    submenu: HashMap<i32, TrayMenuItem>,
    tx: mpsc::Sender<i32>,
}

#[derive(Debug)]
enum TrayMenuWidget {
    Separator(SeparatorMenuItem),
    Standard(MenuItem),
    Checkbox(CheckMenuItem),
}

impl TrayMenuItem {
    fn new(info: &MenuItemInfo, tx: mpsc::Sender<i32>) -> Self {
        let mut submenu = HashMap::new();
        let menu = Menu::new();

        macro_rules! add_submenu {
            ($menu:expr, $widget:expr) => {
                if !info.submenu.is_empty() {
                    for sub_item in &info.submenu {
                        let sub_item = TrayMenuItem::new(sub_item, tx.clone());
                        call!($menu, add, sub_item.widget);
                        submenu.insert(sub_item.id, sub_item);
                    }

                    $widget.set_submenu(Some(&menu));
                }
            };
        }

        let widget = match (info.menu_type, info.toggle_type) {
            (MenuType::Separator, _) => TrayMenuWidget::Separator(SeparatorMenuItem::new()),
            (MenuType::Standard, ToggleType::Checkmark) => {
                let widget = CheckMenuItem::builder()
                    .label(info.label.as_str())
                    .visible(info.visible)
                    .sensitive(info.enabled)
                    .active(info.toggle_state == ToggleState::On)
                    .build();

                add_submenu!(menu, widget);

                {
                    let tx = tx.clone();
                    let id = info.id;

                    widget.connect_activate(move |_item| {
                        try_send!(tx, id);
                    });
                }

                TrayMenuWidget::Checkbox(widget)
            }
            (MenuType::Standard, _) => {
                let widget = MenuItem::builder()
                    .label(&info.label)
                    .visible(info.visible)
                    .sensitive(info.enabled)
                    .build();

                add_submenu!(menu, widget);

                {
                    let tx = tx.clone();
                    let id = info.id;

                    widget.connect_activate(move |_item| {
                        try_send!(tx, id);
                    });
                }

                TrayMenuWidget::Standard(widget)
            }
        };

        Self {
            id: info.id,
            widget,
            menu_widget: menu,
            submenu,
            tx,
        }
    }

    /// Applies a diff to this submenu item.
    ///
    /// This is called recursively,
    /// applying the submenu diffs to any further submenu items.
    fn apply_diff(&mut self, diff: MenuItemDiff) {
        if let Some(label) = diff.label {
            match &self.widget {
                TrayMenuWidget::Separator(widget) => widget.set_label(&label),
                TrayMenuWidget::Standard(widget) => widget.set_label(&label),
                TrayMenuWidget::Checkbox(widget) => widget.set_label(&label),
            }
        }

        // TODO: Image support
        // if let Some(icon_name) = diff.icon_name {
        //
        // }

        if let Some(enabled) = diff.enabled {
            match &self.widget {
                TrayMenuWidget::Separator(widget) => widget.set_sensitive(enabled),
                TrayMenuWidget::Standard(widget) => widget.set_sensitive(enabled),
                TrayMenuWidget::Checkbox(widget) => widget.set_sensitive(enabled),
            }
        }

        if let Some(visible) = diff.visible {
            match &self.widget {
                TrayMenuWidget::Separator(widget) => widget.set_visible(visible),
                TrayMenuWidget::Standard(widget) => widget.set_visible(visible),
                TrayMenuWidget::Checkbox(widget) => widget.set_visible(visible),
            }
        }

        if let Some(toggle_state) = diff.toggle_state {
            if let TrayMenuWidget::Checkbox(widget) = &self.widget {
                widget.set_active(toggle_state == ToggleState::On);
            }
        }

        for sub_diff in diff.submenu {
            match sub_diff {
                Diff::Add(info) => {
                    let menu_item = TrayMenuItem::new(&info, self.tx.clone());
                    call!(self.menu_widget, add, menu_item.widget);

                    if let TrayMenuWidget::Standard(widget) = &self.widget {
                        widget.set_submenu(Some(&self.menu_widget));
                    }

                    self.submenu.insert(menu_item.id, menu_item);
                }
                Diff::Update(id, diff) => {
                    if let Some(sub) = self.submenu.get_mut(&id) {
                        sub.apply_diff(diff);
                    }
                }
                Diff::Remove(id) => {
                    if let Some(sub) = self.submenu.remove(&id) {
                        call!(self.menu_widget, remove, sub.widget);
                    }
                    if let TrayMenuWidget::Standard(widget) = &self.widget {
                        if self.submenu.is_empty() {
                            widget.set_submenu(None::<&Menu>);
                        }
                    }
                }
            }
        }
    }
}
