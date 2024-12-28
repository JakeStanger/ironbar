use super::open_state::OpenState;
use crate::channels::AsyncSenderExt;
use crate::clients::wayland::ToplevelInfo;
use crate::config::{BarPosition, TruncateMode};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::image::ImageProvider;
use crate::modules::launcher::{ItemEvent, LauncherUpdate};
use crate::modules::ModuleUpdateEvent;
use crate::read_lock;
use glib::Propagation;
use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY};
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image, Label, Orientation};
use indexmap::IndexMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::RwLock;
use tokio::sync::mpsc::Sender;
use tracing::error;

#[derive(Debug, Clone)]
pub struct Item {
    pub app_id: String,
    pub favorite: bool,
    pub open_state: OpenState,
    pub windows: IndexMap<usize, Window>,
    pub name: String,
}

impl Item {
    pub fn new(app_id: String, open_state: OpenState, favorite: bool) -> Self {
        Self {
            app_id,
            favorite,
            open_state,
            windows: IndexMap::new(),
            name: String::new(),
        }
    }

    /// Merges the provided node into this launcher item
    pub fn merge_toplevel(&mut self, info: ToplevelInfo) -> Window {
        let id = info.id;

        if self.windows.is_empty() {
            self.name.clone_from(&info.title);
        }

        let window = Window::from(info);
        self.windows.insert(id, window.clone());

        self.recalculate_open_state();

        window
    }

    pub fn unmerge_toplevel(&mut self, info: &ToplevelInfo) {
        self.windows.shift_remove(&info.id);
        self.recalculate_open_state();
    }

    pub fn set_window_name(&mut self, window_id: usize, name: String) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            if let OpenState::Open { focused: true, .. } = window.open_state {
                self.name.clone_from(&name);
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
                .map(|(_, win)| &win.open_state)
                .collect::<Vec<_>>(),
        );
        self.open_state = new_state;
    }
}

impl From<ToplevelInfo> for Item {
    fn from(info: ToplevelInfo) -> Self {
        let id = info.id;
        let name = info.title.clone();
        let app_id = info.app_id.clone();
        let open_state = OpenState::from(&info);

        let mut windows = IndexMap::new();
        let window = Window::from(info);
        windows.insert(id, window);

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
    fn from(info: ToplevelInfo) -> Self {
        let open_state = OpenState::from(&info);

        Self {
            id: info.id,
            name: info.title,
            open_state,
        }
    }
}

pub struct MenuState {
    pub num_windows: usize,
}

pub struct ItemButton {
    pub button: ImageTextButton,
    pub persistent: bool,
    pub show_names: bool,
    pub menu_state: Rc<RwLock<MenuState>>,
}

#[derive(Clone, Copy)]
pub struct AppearanceOptions {
    pub show_names: bool,
    pub show_icons: bool,
    pub icon_size: i32,
    pub truncate: TruncateMode,
}

impl ItemButton {
    pub fn new(
        item: &Item,
        appearance: AppearanceOptions,
        icon_theme: &IconTheme,
        bar_position: BarPosition,
        tx: &Sender<ModuleUpdateEvent<LauncherUpdate>>,
        controller_tx: &Sender<ItemEvent>,
    ) -> Self {
        let button = ImageTextButton::new();

        if appearance.show_names {
            button.label.set_label(&item.name);
            button.label.truncate(appearance.truncate);
        }

        if appearance.show_icons {
            let input = if item.app_id.is_empty() {
                item.name.clone()
            } else {
                item.app_id.clone()
            };
            let image = ImageProvider::parse(&input, icon_theme, true, appearance.icon_size);
            if let Some(image) = image {
                button.set_always_show_image(true);

                if let Err(err) = image.load_into_image(&button.image) {
                    error!("{err:?}");
                }
            };
        }

        button.add_class("item");

        if item.favorite {
            button.add_class("favorite");
        }
        if item.open_state.is_open() {
            button.add_class("open");
        }
        if item.open_state.is_focused() {
            button.add_class("focused");
        }

        let menu_state = Rc::new(RwLock::new(MenuState {
            num_windows: item.windows.len(),
        }));

        {
            let app_id = item.app_id.clone();
            let tx = controller_tx.clone();
            let menu_state = menu_state.clone();

            button.connect_button_release_event(move |button, event| {
                if event.button() == BUTTON_PRIMARY {
                    // lazy check :| TODO: Improve this
                    let style_context = button.style_context();
                    if style_context.has_class("open") {
                        let menu_state = read_lock!(menu_state);

                        if style_context.has_class("focused") && menu_state.num_windows == 1 {
                            tx.send_spawn(ItemEvent::MinimizeItem(app_id.clone()));
                        } else {
                            tx.send_spawn(ItemEvent::FocusItem(app_id.clone()));
                        }
                    } else {
                        tx.send_spawn(ItemEvent::OpenItem(app_id.clone()));
                    }
                } else if event.button() == BUTTON_MIDDLE {
                    tx.send_spawn(ItemEvent::OpenItem(app_id.clone()));
                }

                Propagation::Proceed
            });
        }

        {
            let app_id = item.app_id.clone();
            let tx = tx.clone();
            let menu_state = menu_state.clone();

            button.connect_enter_notify_event(move |button, _| {
                let menu_state = read_lock!(menu_state);

                if menu_state.num_windows > 1 {
                    tx.send_update_spawn(LauncherUpdate::Hover(app_id.clone()));

                    tx.send_spawn(ModuleUpdateEvent::OpenPopupAt(
                        button.geometry(bar_position.orientation()),
                    ));
                } else {
                    tx.send_spawn(ModuleUpdateEvent::ClosePopup);
                }

                Propagation::Proceed
            });
        }

        {
            let tx = tx.clone();

            button.connect_leave_notify_event(move |button, ev| {
                const THRESHOLD: f64 = 5.0;

                let alloc = button.allocation();

                let (x, y) = ev.position();

                let close = match bar_position {
                    BarPosition::Top => y + THRESHOLD < f64::from(alloc.height()),
                    BarPosition::Bottom => y > THRESHOLD,
                    BarPosition::Left => x + THRESHOLD < f64::from(alloc.width()),
                    BarPosition::Right => x > THRESHOLD,
                };

                if close {
                    tx.send_spawn(ModuleUpdateEvent::ClosePopup);
                }

                Propagation::Proceed
            });
        }

        button.show_all();

        Self {
            button,
            persistent: item.favorite,
            show_names: appearance.show_names,
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

#[derive(Debug, Clone)]
pub struct ImageTextButton {
    pub(crate) button: Button,
    pub(crate) label: Label,
    image: Image,
}

impl ImageTextButton {
    pub(crate) fn new() -> Self {
        let button = Button::new();
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let label = Label::new(None);
        let image = Image::new();

        button.add(&container);

        container.add(&image);
        container.add(&label);

        ImageTextButton {
            button,
            label,
            image,
        }
    }
}

impl Deref for ImageTextButton {
    type Target = Button;

    fn deref(&self) -> &Self::Target {
        &self.button
    }
}
