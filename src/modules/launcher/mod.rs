mod item;
mod popup;

use crate::collection::Collection;
use crate::modules::launcher::item::{ButtonConfig, LauncherItem, LauncherWindow};
use crate::modules::launcher::popup::Popup;
use crate::modules::{Module, ModuleInfo};
use crate::sway::node::get_open_windows;
use crate::sway::{SwayNode, WindowEvent};
use gtk::prelude::*;
use gtk::{IconTheme, Orientation};
use ksway::{Client, IpcEvent};
use serde::Deserialize;
use std::rc::Rc;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;

#[derive(Debug, Deserialize, Clone)]
pub struct LauncherModule {
    favorites: Option<Vec<String>>,
    #[serde(default = "crate::config::default_false")]
    show_names: bool,
    #[serde(default = "crate::config::default_true")]
    show_icons: bool,

    icon_theme: Option<String>,
}

#[derive(Debug)]
pub enum FocusEvent {
    AppId(String),
    Class(String),
    ConId(i32),
}

type AppId = String;

struct Launcher {
    items: Collection<AppId, LauncherItem>,
    container: gtk::Box,
    button_config: ButtonConfig,
}

impl Launcher {
    fn new(favorites: Vec<String>, container: gtk::Box, button_config: ButtonConfig) -> Self {
        let items = favorites
            .into_iter()
            .map(|app_id| {
                (
                    app_id.clone(),
                    LauncherItem::new(app_id, true, &button_config),
                )
            })
            .collect::<Collection<_, _>>();

        for item in &items {
            container.add(&item.button);
        }

        Self {
            items,
            container,
            button_config,
        }
    }

    /// Adds a new window to the launcher.
    /// This gets added to an existing group
    /// if an instance of the program is already open.
    fn add_window(&mut self, window: SwayNode) {
        let id = window.get_id().to_string();

        if let Some(item) = self.items.get_mut(&id) {
            let mut state = item.state.write().unwrap();
            state.open = true;
            state.focused = window.focused || state.focused;
            state.urgent = window.urgent || state.urgent;
            state.is_xwayland = window.is_xwayland();

            item.update_button_classes(&state);

            let mut windows = item.windows.lock().unwrap();

            windows.insert(
                window.id,
                LauncherWindow {
                    con_id: window.id,
                    name: window.name,
                },
            );
        } else {
            let item = LauncherItem::from_node(&window, &self.button_config);

            self.container.add(&item.button);
            self.items.insert(id, item);
        }
    }

    /// Removes a window from the launcher.
    /// This removes it from the group if multiple instances were open.
    /// The button will remain on the launcher if it is favourited.
    fn remove_window(&mut self, window: &SwayNode) {
        let id = window.get_id().to_string();

        let item = self.items.get_mut(&id);

        let remove = if let Some(item) = item {
            let windows = Rc::clone(&item.windows);
            let mut windows = windows.lock().unwrap();

            windows.remove(&window.id);

            if windows.is_empty() {
                let mut state = item.state.write().unwrap();
                state.open = false;
                item.update_button_classes(&state);

                if item.favorite {
                    false
                } else {
                    self.container.remove(&item.button);
                    true
                }
            } else {
                false
            }
        } else {
            false
        };

        if remove {
            self.items.remove(&id);
        }
    }

    fn set_window_focused(&mut self, window: &SwayNode) {
        let id = window.get_id().to_string();

        let currently_focused = self
            .items
            .iter_mut()
            .find(|item| item.state.read().unwrap().focused);
        if let Some(currently_focused) = currently_focused {
            let mut state = currently_focused.state.write().unwrap();
            state.focused = false;
            currently_focused.update_button_classes(&state);
        }

        let item = self.items.get_mut(&id);
        if let Some(item) = item {
            let mut state = item.state.write().unwrap();
            state.focused = true;
            item.update_button_classes(&state);
        }
    }

    fn set_window_title(&mut self, window: SwayNode) {
        let id = window.get_id().to_string();
        let item = self.items.get_mut(&id);

        if let (Some(item), Some(name)) = (item, window.name) {
            let mut windows = item.windows.lock().unwrap();
            if windows.len() == 1 {
                item.set_title(&name, &self.button_config);
            } else {
                windows.get_mut(&window.id).unwrap().name = Some(name);
            }
        }
    }

    fn set_window_urgent(&mut self, window: &SwayNode) {
        let id = window.get_id().to_string();
        let item = self.items.get_mut(&id);

        if let Some(item) = item {
            let mut state = item.state.write().unwrap();
            state.urgent = window.urgent;
            item.update_button_classes(&state);
        }
    }
}

impl Module<gtk::Box> for LauncherModule {
    fn into_widget(self, info: &ModuleInfo) -> gtk::Box {
        let icon_theme = IconTheme::new();

        if let Some(theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(&theme));
        }

        let mut sway = Client::connect().unwrap();

        let popup = Popup::new(
            "popup-launcher",
            info.app,
            info.monitor,
            Orientation::Vertical,
            info.bar_position,
        );
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let (ui_tx, mut ui_rx) = mpsc::channel(32);

        let button_config = ButtonConfig {
            icon_theme,
            show_names: self.show_names,
            show_icons: self.show_icons,
            popup,
            tx: ui_tx,
        };

        let mut launcher = Launcher::new(
            self.favorites.unwrap_or_default(),
            container.clone(),
            button_config,
        );

        let open_windows = get_open_windows(&mut sway);

        for window in open_windows {
            launcher.add_window(window);
        }

        let srx = sway.subscribe(vec![IpcEvent::Window]).unwrap();
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        spawn_blocking(move || loop {
            while let Ok((_, payload)) = srx.try_recv() {
                let payload: WindowEvent = serde_json::from_slice(&payload).unwrap();

                tx.send(payload).unwrap();
            }
            sway.poll().unwrap();
        });

        {
            rx.attach(None, move |event| {
                match event.change.as_str() {
                    "new" => launcher.add_window(event.container),
                    "close" => launcher.remove_window(&event.container),
                    "focus" => launcher.set_window_focused(&event.container),
                    "title" => launcher.set_window_title(event.container),
                    "urgent" => launcher.set_window_urgent(&event.container),
                    _ => {}
                }

                Continue(true)
            });
        }

        spawn(async move {
            let mut sway = Client::connect().unwrap();
            while let Some(event) = ui_rx.recv().await {
                let selector = match event {
                    FocusEvent::AppId(app_id) => format!("[app_id={}]", app_id),
                    FocusEvent::Class(class) => format!("[class={}]", class),
                    FocusEvent::ConId(id) => format!("[con_id={}]", id),
                };

                sway.run(format!("{} focus", selector)).unwrap();
            }
        });

        container
    }
}
