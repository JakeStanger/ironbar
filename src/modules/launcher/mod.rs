mod item;
mod open_state;

use self::item::{AppearanceOptions, Item, ItemButton, Window};
use self::open_state::OpenState;
use super::{Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext};
use crate::clients::wayland::{self, ToplevelEvent};
use crate::config::CommonConfig;
use crate::desktop_file::find_desktop_file;
use crate::{arc_mut, glib_recv, lock, send_async, spawn, try_send, write_lock};
use color_eyre::{Help, Report};
use gtk::prelude::*;
use gtk::{Button, Orientation};
use indexmap::IndexMap;
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, trace};

#[derive(Debug, Deserialize, Clone)]
pub struct LauncherModule {
    /// List of app IDs (or classes) to always show regardless of open state,
    /// in the order specified.
    favorites: Option<Vec<String>>,
    /// Whether to show application names on the bar.
    #[serde(default = "crate::config::default_false")]
    show_names: bool,
    /// Whether to show application icons on the bar.
    #[serde(default = "crate::config::default_true")]
    show_icons: bool,

    #[serde(default = "default_icon_size")]
    icon_size: i32,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_icon_size() -> i32 {
    32
}

#[derive(Debug, Clone)]
pub enum LauncherUpdate {
    /// Adds item
    AddItem(Item),
    /// Adds window to item with `app_id`
    AddWindow(String, Window),
    /// Removes item with `app_id`
    RemoveItem(String),
    /// Removes window from item with `app_id`.
    RemoveWindow(String, usize),
    /// Sets title for `app_id`
    Title(String, usize, String),
    /// Marks the item with `app_id` as focused or not focused
    Focus(String, bool),
    /// Declares the item with `app_id` has been hovered over
    Hover(String),
}

#[derive(Debug)]
pub enum ItemEvent {
    FocusItem(String),
    FocusWindow(usize),
    OpenItem(String),
}

enum ItemOrWindow {
    Item(Item),
    Window(Window),
}

enum ItemOrWindowId {
    Item,
    Window,
}

impl Module<gtk::Box> for LauncherModule {
    type SendMessage = LauncherUpdate;
    type ReceiveMessage = ItemEvent;

    fn name() -> &'static str {
        "launcher"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> crate::Result<()> {
        let items = self
            .favorites
            .as_ref()
            .map_or_else(IndexMap::new, |favorites| {
                favorites
                    .iter()
                    .map(|app_id| {
                        (
                            app_id.to_string(),
                            Item::new(app_id.to_string(), OpenState::Closed, true),
                        )
                    })
                    .collect::<IndexMap<_, _>>()
            });

        let items = arc_mut!(items);

        let items2 = Arc::clone(&items);

        let tx = context.tx.clone();
        let tx2 = context.tx.clone();

        let wl = context.client::<wayland::Client>();
        spawn(async move {
            let items = items2;
            let tx = tx2;

            let mut wlrx = wl.subscribe_toplevels();
            let handles = wl.toplevel_info_all();

            for info in handles {
                let mut items = lock!(items);
                let item = items.get_mut(&info.app_id);
                match item {
                    Some(item) => {
                        item.merge_toplevel(info.clone());
                    }
                    None => {
                        items.insert(info.app_id.clone(), Item::from(info.clone()));
                    }
                }
            }

            {
                let items = lock!(items);
                let items = items.iter();
                for (_, item) in items {
                    try_send!(
                        tx,
                        ModuleUpdateEvent::Update(LauncherUpdate::AddItem(item.clone()))
                    );
                }
            }

            let send_update = |update: LauncherUpdate| tx.send(ModuleUpdateEvent::Update(update));

            while let Ok(event) = wlrx.recv().await {
                trace!("event: {:?}", event);

                match event {
                    ToplevelEvent::New(info) => {
                        let app_id = info.app_id.clone();

                        let new_item = {
                            let mut items = lock!(items);
                            let item = items.get_mut(&info.app_id);
                            match item {
                                None => {
                                    let item: Item = info.into();

                                    items.insert(app_id.clone(), item.clone());

                                    ItemOrWindow::Item(item)
                                }
                                Some(item) => {
                                    let window = item.merge_toplevel(info);
                                    ItemOrWindow::Window(window)
                                }
                            }
                        };

                        match new_item {
                            ItemOrWindow::Item(item) => {
                                send_update(LauncherUpdate::AddItem(item)).await
                            }
                            ItemOrWindow::Window(window) => {
                                send_update(LauncherUpdate::AddWindow(app_id, window)).await
                            }
                        }?;
                    }
                    ToplevelEvent::Update(info) => {
                        if let Some(item) = lock!(items).get_mut(&info.app_id) {
                            item.set_window_focused(info.id, info.focused);
                            item.set_window_name(info.id, info.title.clone());
                        }

                        send_update(LauncherUpdate::Focus(info.app_id.clone(), info.focused))
                            .await?;
                        send_update(LauncherUpdate::Title(
                            info.app_id.clone(),
                            info.id,
                            info.title.clone(),
                        ))
                        .await?;
                    }
                    ToplevelEvent::Remove(info) => {
                        let remove_item = {
                            let mut items = lock!(items);
                            let item = items.get_mut(&info.app_id);
                            match item {
                                Some(item) => {
                                    item.unmerge_toplevel(&info);

                                    if item.windows.is_empty() {
                                        items.remove(&info.app_id);
                                        Some(ItemOrWindowId::Item)
                                    } else {
                                        Some(ItemOrWindowId::Window)
                                    }
                                }
                                None => None,
                            }
                        };

                        match remove_item {
                            Some(ItemOrWindowId::Item) => {
                                send_update(LauncherUpdate::RemoveItem(info.app_id.clone()))
                                    .await?;
                            }
                            Some(ItemOrWindowId::Window) => {
                                send_update(LauncherUpdate::RemoveWindow(
                                    info.app_id.clone(),
                                    info.id,
                                ))
                                .await?;
                            }
                            None => {}
                        };
                    }
                }
            }

            Ok::<(), Report>(())
        });

        // listen to ui events
        let wl = context.client::<wayland::Client>();
        spawn(async move {
            while let Some(event) = rx.recv().await {
                if let ItemEvent::OpenItem(app_id) = event {
                    find_desktop_file(&app_id).map_or_else(
                        || error!("Could not find desktop file for {}", app_id),
                        |file| {
                            if let Err(err) = Command::new("gtk-launch")
                                .arg(
                                    file.file_name()
                                        .expect("File segment missing from path to desktop file"),
                                )
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn()
                            {
                                error!(
                                    "{:?}",
                                    Report::new(err)
                                        .wrap_err("Failed to run gtk-launch command.")
                                        .suggestion("Perhaps the desktop file is invalid?")
                                );
                            }
                        },
                    );
                } else {
                    send_async!(tx, ModuleUpdateEvent::ClosePopup);

                    let id = match event {
                        ItemEvent::FocusItem(app_id) => {
                            lock!(items).get(&app_id).and_then(|item| {
                                item.windows
                                    .iter()
                                    .find(|(_, win)| !win.open_state.is_focused())
                                    .or_else(|| item.windows.first())
                                    .map(|(_, win)| win.id)
                            })
                        }
                        ItemEvent::FocusWindow(id) => Some(id),
                        ItemEvent::OpenItem(_) => unreachable!(),
                    };

                    if let Some(id) = id {
                        if let Some(window) = lock!(items)
                            .iter()
                            .find_map(|(_, item)| item.windows.get(&id))
                        {
                            debug!("Focusing window {id}: {}", window.name);
                            wl.toplevel_focus(window.id);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> crate::Result<ModuleParts<gtk::Box>> {
        let icon_theme = info.icon_theme;

        let container = gtk::Box::new(info.bar_position.get_orientation(), 0);

        {
            let container = container.clone();
            let icon_theme = icon_theme.clone();

            let controller_tx = context.controller_tx.clone();

            let appearance_options = AppearanceOptions {
                show_names: self.show_names,
                show_icons: self.show_icons,
                icon_size: self.icon_size,
            };

            let show_names = self.show_names;
            let bar_position = info.bar_position;

            let mut buttons = IndexMap::<String, ItemButton>::new();

            let tx = context.tx.clone();
            let rx = context.subscribe();
            glib_recv!(rx, event => {
                match event {
                    LauncherUpdate::AddItem(item) => {
                        debug!("Adding item with id {}", item.app_id);

                        if let Some(button) = buttons.get(&item.app_id) {
                            button.set_open(true);
                        } else {
                            let button = ItemButton::new(
                                &item,
                                appearance_options,
                                &icon_theme,
                                bar_position,
                                &tx,
                                &controller_tx,
                            );

                            container.add(&button.button);
                            buttons.insert(item.app_id, button);
                        }
                    }
                    LauncherUpdate::AddWindow(app_id, win) => {
                        if let Some(button) = buttons.get(&app_id) {
                            button.set_open(true);
                            button.set_focused(win.open_state.is_focused());

                            let mut menu_state = write_lock!(button.menu_state);
                            menu_state.num_windows += 1;
                        }
                    }
                    LauncherUpdate::RemoveItem(app_id) => {
                        debug!("Removing item with id {}", app_id);

                        if let Some(button) = buttons.get(&app_id) {
                            if button.persistent {
                                button.set_open(false);
                                if button.show_names {
                                    button.button.set_label(&app_id);
                                }
                            } else {
                                container.remove(&button.button);
                                buttons.remove(&app_id);
                            }
                        }
                    }
                    LauncherUpdate::RemoveWindow(app_id, win_id) => {
                        debug!("Removing window {win_id} with id {app_id}");

                        if let Some(button) = buttons.get(&app_id) {
                            button.set_focused(false);

                            let mut menu_state = write_lock!(button.menu_state);
                            menu_state.num_windows -= 1;
                        }
                    }
                    LauncherUpdate::Focus(app_id, focus) => {
                        debug!("Changing focus to {} on item with id {}", focus, app_id);

                        if let Some(button) = buttons.get(&app_id) {
                            button.set_focused(focus);
                        }
                    }
                    LauncherUpdate::Title(app_id, _, name) => {
                        debug!("Updating title for item with id {}: {:?}", app_id, name);

                        if show_names {
                            if let Some(button) = buttons.get(&app_id) {
                                button.button.set_label(&name);
                            }
                        }
                    }
                    LauncherUpdate::Hover(_) => {}
                };
            });
        }

        let rx = context.subscribe();
        let popup = self
            .into_popup(context.controller_tx, rx, info)
            .into_popup_parts(vec![]); // since item buttons are dynamic, they pass their geometry directly

        Ok(ModuleParts {
            widget: container,
            popup,
        })
    }

    fn into_popup(
        self,
        controller_tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        const MAX_WIDTH: i32 = 250;

        let container = gtk::Box::new(Orientation::Vertical, 0);

        // we need some content to force the container to have a size
        let placeholder = Button::with_label("PLACEHOLDER");
        placeholder.set_width_request(MAX_WIDTH);
        container.add(&placeholder);

        let mut buttons = IndexMap::<String, IndexMap<usize, Button>>::new();

        {
            let container = container.clone();
            glib_recv!(rx, event => {
                match event {
                    LauncherUpdate::AddItem(item) => {
                        let app_id = item.app_id.clone();
                        trace!("Adding item with id '{app_id}' to the popup: {item:?}");

                        let window_buttons = item
                            .windows
                            .into_iter()
                            .map(|(_, win)| {
                                let button = Button::builder()
                                    .label(clamp(&win.name))
                                    .height_request(40)
                                    .build();

                                {
                                    let tx = controller_tx.clone();
                                    button.connect_clicked(move |_| {
                                        try_send!(tx, ItemEvent::FocusWindow(win.id));
                                    });
                                }

                                (win.id, button)
                            })
                            .collect();

                        buttons.insert(app_id, window_buttons);
                    }
                    LauncherUpdate::AddWindow(app_id, win) => {
                        debug!(
                            "Adding new window to popup for '{app_id}': '{}' ({})",
                            win.name, win.id
                        );

                        if let Some(buttons) = buttons.get_mut(&app_id) {
                            let button = Button::builder()
                                .height_request(40)
                                .label(clamp(&win.name))
                                .build();

                            {
                                let tx = controller_tx.clone();
                                button.connect_clicked(move |_button| {
                                    try_send!(tx, ItemEvent::FocusWindow(win.id));
                                });
                            }

                            buttons.insert(win.id, button);
                        }
                    }
                    LauncherUpdate::RemoveWindow(app_id, win_id) => {
                        debug!("Removing window from popup for '{app_id}': {win_id}");

                        if let Some(buttons) = buttons.get_mut(&app_id) {
                            buttons.remove(&win_id);
                        }
                    }
                    LauncherUpdate::Title(app_id, win_id, title) => {
                        debug!(
                            "Updating window title on popup for '{app_id}'/{win_id} to '{title}'"
                        );

                        if let Some(buttons) = buttons.get_mut(&app_id) {
                            if let Some(button) = buttons.get(&win_id) {
                                button.set_label(&title);
                            }
                        }
                    }
                    LauncherUpdate::Hover(app_id) => {
                        // empty current buttons
                        for child in container.children() {
                            container.remove(&child);
                        }

                        // add app's buttons
                        if let Some(buttons) = buttons.get(&app_id) {
                            for (_, button) in buttons {
                                button.style_context().add_class("popup-item");
                                container.add(button);
                            }

                            container.show_all();
                            container.set_width_request(MAX_WIDTH);
                        }
                    }
                    _ => {}
                }
            });
        }

        Some(container)
    }
}

/// Clamps a string at 24 characters.
///
/// This is a hacky number derived from
/// "what fits inside the 250px popup"
/// and probably won't hold up with wide fonts.
///
/// TODO: Migrate this to truncate system
///
fn clamp(str: &str) -> String {
    const MAX_CHARS: usize = 24;

    if str.len() > MAX_CHARS {
        str.chars().take(MAX_CHARS - 3).collect::<String>() + "..."
    } else {
        str.to_string()
    }
}
