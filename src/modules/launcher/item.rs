use super::open_state::OpenState;
use crate::channels::AsyncSenderExt;
use crate::clients::wayland::{Buffer, ToplevelInfo};
use crate::config::{BarPosition, TruncateMode};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt, MouseButton};
use crate::modules::launcher::{ItemEvent, LauncherUpdate};
use crate::modules::{ModuleUpdateEvent, PopupButton};
use crate::{image, read_lock};
use gtk::prelude::*;
use gtk::{
    Align, Button, ContentFit, EventControllerMotion, Justification, Label, Orientation, Picture,
};
use indexmap::IndexMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::RwLock;
use tokio::sync::mpsc::Sender;

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

    pub fn set_window_buffer(&mut self, window_id: usize, buffer: Option<Buffer>) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.preview_buffer = buffer;
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
    pub preview_buffer: Option<Buffer>,
}

impl From<ToplevelInfo> for Window {
    fn from(info: ToplevelInfo) -> Self {
        let open_state = OpenState::from(&info);

        Self {
            id: info.id,
            name: info.title,
            open_state,
            preview_buffer: None,
        }
    }
}

pub struct MenuState {
    pub num_windows: usize,
}

#[derive(Clone)]
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
    pub show_previews: bool,
    pub icon_size: i32,
    pub truncate: TruncateMode,
    pub orientation: Orientation,
    pub justify: Justification,
}

impl ItemButton {
    pub fn new(
        item: &Item,
        appearance: AppearanceOptions,
        image_provider: image::Provider,
        bar_position: BarPosition,
        tx: &Sender<ModuleUpdateEvent<LauncherUpdate>>,
        controller_tx: &Sender<ItemEvent>,
    ) -> Self {
        let button = ImageTextButton::new(appearance.orientation);

        if appearance.show_names {
            button.label.set_label(&item.name);
            button.label.truncate(appearance.truncate);
            button.label.set_justify(appearance.justify);
        }

        if appearance.show_icons {
            let input = if item.app_id.is_empty() {
                item.name.clone()
            } else {
                item.app_id.clone()
            };

            let button = button.clone();
            glib::spawn_future_local(async move {
                image_provider
                    .load_into_picture_silent(&input, appearance.icon_size, true, &button.picture)
                    .await;
            });
        }

        button.add_css_class("item");

        if item.favorite {
            button.add_css_class("favorite");
        }
        if item.open_state.is_open() {
            button.add_css_class("open");
        }
        if item.open_state.is_focused() {
            button.add_css_class("focused");
        }

        let menu_state = Rc::new(RwLock::new(MenuState {
            num_windows: item.windows.len(),
        }));

        {
            let app_id = item.app_id.clone();
            let tx = controller_tx.clone();
            let menu_state = menu_state.clone();

            let button2 = button.clone();
            button.connect_pressed(MouseButton::Primary, move || {
                // lazy check :| TODO: Improve this
                if button2.has_css_class("open") {
                    let menu_state = read_lock!(menu_state);

                    if button2.has_css_class("focused") && menu_state.num_windows == 1 {
                        tx.send_spawn(ItemEvent::MinimizeItem(app_id.clone()));
                    } else {
                        tx.send_spawn(ItemEvent::FocusItem(app_id.clone()));
                    }
                } else {
                    tx.send_spawn(ItemEvent::OpenItem(app_id.clone()));
                }
            });
        }

        {
            let app_id = item.app_id.clone();
            let tx = controller_tx.clone();

            button.connect_pressed(MouseButton::Middle, move || {
                tx.send_spawn(ItemEvent::OpenItem(app_id.clone()));
            });
        }

        let event_controller = EventControllerMotion::new();

        {
            let app_id = item.app_id.clone();
            let tx = tx.clone();
            let menu_state = menu_state.clone();

            let button = button.clone();

            event_controller.connect_enter(move |_, _, _| {
                let menu_state = read_lock!(menu_state);

                if (appearance.show_previews && menu_state.num_windows > 0)
                    || menu_state.num_windows > 1
                {
                    tx.send_update_spawn(LauncherUpdate::Hover(app_id.clone()));
                    tx.send_spawn(ModuleUpdateEvent::OpenPopup(button.popup_id()));
                } else {
                    tx.send_spawn(ModuleUpdateEvent::ClosePopup);
                }
            });
        }

        {
            let tx = tx.clone();
            let button = button.clone();

            // TODO: Evaluate: do we need this, or can we fix it for edge items?
            event_controller.connect_leave(move |controller| {
                const THRESHOLD: f64 = 5.0;

                let Some(ev) = controller.current_event() else {
                    return;
                };

                let alloc = button.allocation();

                let (x, y) = ev.position().unwrap_or_default();

                let close = match bar_position {
                    BarPosition::Top => y + THRESHOLD < f64::from(alloc.height()),
                    BarPosition::Bottom => y > THRESHOLD,
                    BarPosition::Left => x + THRESHOLD < f64::from(alloc.width()),
                    BarPosition::Right => x > THRESHOLD,
                };

                if close {
                    tx.send_spawn(ModuleUpdateEvent::ClosePopup);
                }
            });
        }

        button.add_controller(event_controller);

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
        if toggle {
            self.button.add_css_class(class);
        } else {
            self.button.remove_css_class(class);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageTextButton {
    pub(crate) button: Button,
    pub(crate) label: Label,
    pub(crate) picture: Picture,
}

impl ImageTextButton {
    pub(crate) fn new(orientation: Orientation) -> Self {
        let button = Button::new();
        button.ensure_popup_id(); // all launcher buttons could be used in popups

        let container = gtk::Box::new(orientation, 0);

        let label = Label::new(None);

        let picture = Picture::builder()
            .content_fit(ContentFit::ScaleDown)
            .build();

        container.append(&picture);
        container.append(&label);

        button.set_child(Some(&container));
        container.set_halign(Align::Center);
        container.set_valign(Align::Center);

        ImageTextButton {
            button,
            label,
            picture,
        }
    }
}

impl Deref for ImageTextButton {
    type Target = Button;

    fn deref(&self) -> &Self::Target {
        &self.button
    }
}
