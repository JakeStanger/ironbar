mod item;
mod open_state;
mod popup;

use crate::collection::Collection;
use crate::modules::launcher::item::{ButtonConfig, LauncherItem, LauncherWindow};
use crate::modules::launcher::open_state::OpenState;
use crate::modules::launcher::popup::Popup;
use crate::modules::{Module, ModuleInfo};
use crate::sway::{get_client, SwayNode};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{IconTheme, Orientation};
use serde::Deserialize;
use std::rc::Rc;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tracing::debug;

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
    fn add_window(&mut self, node: SwayNode) {
        let id = node.get_id().to_string();

        debug!("Adding window with ID {}", id);

        if let Some(item) = self.items.get_mut(&id) {
            let mut state = item
                .state
                .write()
                .expect("Failed to get write lock on state");
            let new_open_state = OpenState::from_node(&node);
            state.open_state = OpenState::merge_states(vec![&state.open_state, &new_open_state]);
            state.is_xwayland = node.is_xwayland();

            item.update_button_classes(&state);

            let mut windows = item
                .windows
                .write()
                .expect("Failed to get write lock on windows");

            windows.insert(
                node.id,
                LauncherWindow {
                    con_id: node.id,
                    name: node.name,
                    open_state: new_open_state,
                },
            );
        } else {
            let item = LauncherItem::from_node(&node, &self.button_config);

            self.container.add(&item.button);
            self.items.insert(id, item);
        }
    }

    /// Removes a window from the launcher.
    /// This removes it from the group if multiple instances were open.
    /// The button will remain on the launcher if it is favourited.
    fn remove_window(&mut self, window: &SwayNode) {
        let id = window.get_id().to_string();

        debug!("Removing window with ID {}", id);

        let item = self.items.get_mut(&id);

        let remove = if let Some(item) = item {
            let windows = Rc::clone(&item.windows);
            let mut windows = windows
                .write()
                .expect("Failed to get write lock on windows");

            windows.remove(&window.id);

            if windows.is_empty() {
                let mut state = item.state.write().expect("Failed to get lock on windows");
                state.open_state = OpenState::Closed;
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

    /// Unfocuses the currently focused window
    /// and focuses the newly focused one.
    fn set_window_focused(&mut self, node: &SwayNode) {
        let id = node.get_id().to_string();

        debug!("Setting window with ID {} focused", id);

        let prev_focused = self.items.iter_mut().find(|item| {
            item.state
                .read()
                .expect("Failed to get read lock on state")
                .open_state
                .is_focused()
        });

        if let Some(prev_focused) = prev_focused {
            let mut state = prev_focused
                .state
                .write()
                .expect("Failed to get write lock on state");

            // if a window from the same item took focus,
            // we don't need to unfocus the item.
            if prev_focused.app_id != id {
                prev_focused.set_open_state(OpenState::open(), &mut state);
                prev_focused.update_button_classes(&state);
            }
        }

        let item = self.items.get_mut(&id);
        if let Some(item) = item {
            let mut state = item
                .state
                .write()
                .expect("Failed to get write lock on state");
            item.set_window_open_state(node.id, OpenState::focused(), &mut state);
            item.update_button_classes(&state);
        }
    }

    /// Updates the window title for the given node.
    fn set_window_title(&mut self, window: SwayNode) {
        let id = window.get_id().to_string();
        let item = self.items.get_mut(&id);

        debug!("Updating title for window with ID {}", id);

        if let (Some(item), Some(name)) = (item, window.name) {
            let mut windows = item
                .windows
                .write()
                .expect("Failed to get write lock on windows");
            if windows.len() == 1 {
                item.set_title(&name, &self.button_config);
            } else if let Some(window) = windows.get_mut(&window.id) {
                window.name = Some(name);
            } else {
                // This should never happen
                // But makes more sense to wipe title than keep old one in case of error
                item.set_title("", &self.button_config);
            }
        }
    }

    /// Updates the window urgency based on the given node.
    fn set_window_urgent(&mut self, node: &SwayNode) {
        let id = node.get_id().to_string();
        let item = self.items.get_mut(&id);

        debug!(
            "Setting urgency to {} for window with ID {}",
            node.urgent, id
        );

        if let Some(item) = item {
            let mut state = item
                .state
                .write()
                .expect("Failed to get write lock on state");

            item.set_window_open_state(node.id, OpenState::urgent(node.urgent), &mut state);
            item.update_button_classes(&state);
        }
    }
}

impl Module<gtk::Box> for LauncherModule {
    fn into_widget(self, info: &ModuleInfo) -> Result<gtk::Box> {
        let icon_theme = IconTheme::new();

        if let Some(theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(&theme));
        }

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

        let open_windows = {
            let sway = get_client();
            let mut sway = sway.lock().expect("Failed to get lock on Sway IPC client");
            sway.get_open_windows()
        }?;

        for window in open_windows {
            launcher.add_window(window);
        }

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        spawn_blocking(move || {
            let srx = {
                let sway = get_client();
                let mut sway = sway.lock().expect("Failed to get lock on Sway IPC client");
                sway.subscribe_window()
            };

            while let Ok(payload) = srx.recv() {
                tx.send(payload)
                    .expect("Failed to send window event payload");
            }
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
            let sway = get_client();

            while let Some(event) = ui_rx.recv().await {
                let selector = match event {
                    FocusEvent::AppId(app_id) => format!("[app_id={}]", app_id),
                    FocusEvent::Class(class) => format!("[class={}]", class),
                    FocusEvent::ConId(id) => format!("[con_id={}]", id),
                };
                let mut sway = sway.lock().expect("Failed to get lock on Sway IPC client");
                sway.run(format!("{} focus", selector))?;
            }

            Ok::<(), Report>(())
        });

        Ok(container)
    }
}
