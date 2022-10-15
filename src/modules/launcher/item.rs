use super::open_state::OpenState;
use crate::collection::Collection;
use crate::icon::get_icon;
use crate::modules::launcher::{ItemEvent, LauncherUpdate};
use crate::modules::ModuleUpdateEvent;
use crate::popup::Popup;
use crate::wayland::ToplevelInfo;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image};
use std::rc::Rc;
use std::sync::RwLock;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Item {
    pub app_id: String,
    pub favorite: bool,
    pub open_state: OpenState,
    pub windows: Collection<usize, Window>,
    pub name: String,
}

impl Item {
    pub const fn new(app_id: String, open_state: OpenState, favorite: bool) -> Self {
        Self {
            app_id,
            favorite,
            open_state,
            windows: Collection::new(),
            name: String::new(),
        }
    }

    /// Merges the provided node into this launcher item
    pub fn merge_toplevel(&mut self, node: ToplevelInfo) -> Window {
        let id = node.id;

        if self.windows.is_empty() {
            self.name = node.title.clone();
        }

        let window: Window = node.into();
        self.windows.insert(id, window.clone());

        self.recalculate_open_state();

        window
    }

    pub fn unmerge_toplevel(&mut self, node: &ToplevelInfo) {
        self.windows.remove(&node.id);
        self.recalculate_open_state();
    }

    pub fn set_window_name(&mut self, window_id: usize, name: String) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            if let OpenState::Open { focused: true, .. } = window.open_state {
                self.name = name.clone();
            }

            window.name = name;
        }
    }

    pub fn set_window_focused(&mut self, window_id: usize, focused: bool) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.open_state =
                OpenState::merge_states(&[&window.open_state, &OpenState::focused(focused)]);

            self.recalculate_open_state();
        }
    }

    /// Sets this item's open state
    /// to the merged result of its windows' open states
    fn recalculate_open_state(&mut self) {
        let new_state = OpenState::merge_states(
            &self
                .windows
                .iter()
                .map(|win| &win.open_state)
                .collect::<Vec<_>>(),
        );
        self.open_state = new_state;
    }
}

impl From<ToplevelInfo> for Item {
    fn from(toplevel: ToplevelInfo) -> Self {
        let open_state = OpenState::from_toplevel(&toplevel);
        let name = toplevel.title.clone();
        let app_id = toplevel.app_id.clone();

        let mut windows = Collection::new();
        windows.insert(toplevel.id, toplevel.into());

        Self {
            app_id,
            favorite: false,
            open_state,
            windows,
            name,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Window {
    pub id: usize,
    pub name: String,
    pub open_state: OpenState,
}

impl From<ToplevelInfo> for Window {
    fn from(node: ToplevelInfo) -> Self {
        let open_state = OpenState::from_toplevel(&node);

        Self {
            id: node.id,
            name: node.title,
            open_state,
        }
    }
}

pub struct MenuState {
    pub num_windows: usize,
}

pub struct ItemButton {
    pub button: Button,
    pub persistent: bool,
    pub show_names: bool,
    pub menu_state: Rc<RwLock<MenuState>>,
}

impl ItemButton {
    pub fn new(
        item: &Item,
        show_names: bool,
        show_icons: bool,
        icon_theme: &IconTheme,
        tx: &Sender<ModuleUpdateEvent<LauncherUpdate>>,
        controller_tx: &Sender<ItemEvent>,
    ) -> Self {
        let mut button = Button::builder();

        if show_names {
            button = button.label(&item.name);
        }

        if show_icons {
            let icon = get_icon(icon_theme, &item.app_id, 32);
            if icon.is_some() {
                let image = Image::from_pixbuf(icon.as_ref());
                button = button.image(&image).always_show_image(true);
            }
        }

        let button = button.build();

        let style_context = button.style_context();
        style_context.add_class("item");

        if item.favorite {
            style_context.add_class("favorite");
        }
        if item.open_state.is_open() {
            style_context.add_class("open");
        }
        if item.open_state.is_focused() {
            style_context.add_class("focused");
        }

        {
            let app_id = item.app_id.clone();
            let tx = controller_tx.clone();
            button.connect_clicked(move |button| {
                // lazy check :|
                let style_context = button.style_context();
                if style_context.has_class("open") {
                    tx.try_send(ItemEvent::FocusItem(app_id.clone()))
                        .expect("Failed to send item focus event");
                } else {
                    tx.try_send(ItemEvent::OpenItem(app_id.clone()))
                        .expect("Failed to send item open event");
                }
            });
        }

        let menu_state = Rc::new(RwLock::new(MenuState {
            num_windows: item.windows.len(),
        }));

        {
            let app_id = item.app_id.clone();
            let tx = tx.clone();
            let menu_state = menu_state.clone();

            button.connect_enter_notify_event(move |button, _| {
                let menu_state = menu_state
                    .read()
                    .expect("Failed to get read lock on item menu state");

                if menu_state.num_windows > 1 {
                    tx.try_send(ModuleUpdateEvent::Update(LauncherUpdate::Hover(
                        app_id.clone(),
                    )))
                    .expect("Failed to send item open popup event");

                    tx.try_send(ModuleUpdateEvent::OpenPopup(Popup::button_pos(button)))
                        .expect("Failed to send item open popup event");
                } else {
                    tx.try_send(ModuleUpdateEvent::ClosePopup)
                        .expect("Failed to send item close popup event");
                }

                Inhibit(false)
            });
        }

        button.show_all();

        Self {
            button,
            persistent: item.favorite,
            show_names,
            menu_state,
        }
    }

    pub fn set_open(&self, open: bool) {
        self.update_class("open", open);

        if !open {
            self.set_focused(false);
        }
    }

    pub fn set_focused(&self, focused: bool) {
        self.update_class("focused", focused);
    }

    /// Adds or removes a class to the button based on `toggle`.
    fn update_class(&self, class: &str, toggle: bool) {
        let style_context = self.button.style_context();

        if toggle {
            style_context.add_class(class);
        } else {
            style_context.remove_class(class);
        }
    }
}
