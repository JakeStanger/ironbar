mod item;
mod open_state;

use self::item::{Item, ItemButton, Window};
use self::open_state::OpenState;
use crate::clients::wayland::{self, ToplevelChange};
use crate::config::CommonConfig;
use crate::icon::find_desktop_file;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::{lock, read_lock, try_send, write_lock};
use color_eyre::{Help, Report};
use glib::Continue;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Orientation};
use indexmap::IndexMap;
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
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

    /// Name of the GTK icon theme to use.
    icon_theme: Option<String>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
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
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
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

        let items = Arc::new(Mutex::new(items));

        {
            let items = Arc::clone(&items);
            let tx = tx.clone();
            spawn(async move {
                let wl = wayland::get_client().await;
                let open_windows = read_lock!(wl.toplevels);

                let mut items = lock!(items);

                for (_, (window, _)) in open_windows.clone() {
                    let item = items.get_mut(&window.app_id);
                    match item {
                        Some(item) => {
                            item.merge_toplevel(window);
                        }
                        None => {
                            items.insert(window.app_id.clone(), window.into());
                        }
                    }
                }

                let items = items.iter();
                for (_, item) in items {
                    tx.try_send(ModuleUpdateEvent::Update(LauncherUpdate::AddItem(
                        item.clone(),
                    )))?;
                }

                Ok::<(), Report>(())
            });
        }

        let items2 = Arc::clone(&items);
        spawn(async move {
            let items = items2;

            let mut wlrx = {
                let wl = wayland::get_client().await;
                wl.subscribe_toplevels()
            };

            let send_update = |update: LauncherUpdate| tx.send(ModuleUpdateEvent::Update(update));

            while let Ok(event) = wlrx.recv().await {
                trace!("event: {:?}", event);

                let window = event.toplevel;
                let app_id = window.app_id.clone();

                match event.change {
                    ToplevelChange::New => {
                        let new_item = {
                            let mut items = lock!(items);
                            let item = items.get_mut(&app_id);
                            match item {
                                None => {
                                    let item: Item = window.into();
                                    items.insert(app_id.clone(), item.clone());

                                    ItemOrWindow::Item(item)
                                }
                                Some(item) => {
                                    let window = item.merge_toplevel(window);
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
                    ToplevelChange::Close => {
                        let remove_item = {
                            let mut items = lock!(items);
                            let item = items.get_mut(&app_id);
                            match item {
                                Some(item) => {
                                    item.unmerge_toplevel(&window);

                                    if item.windows.is_empty() {
                                        items.remove(&app_id);
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
                                send_update(LauncherUpdate::RemoveItem(app_id)).await?;
                            }
                            Some(ItemOrWindowId::Window) => {
                                send_update(LauncherUpdate::RemoveWindow(app_id, window.id))
                                    .await?;
                            }
                            None => {}
                        };
                    }
                    ToplevelChange::Focus(focused) => {
                        let mut update_title = false;

                        if focused {
                            if let Some(item) = lock!(items).get_mut(&app_id) {
                                item.set_window_focused(window.id, true);

                                // might be switching focus between windows of same app
                                if item.windows.len() > 1 {
                                    item.set_window_name(window.id, window.title.clone());
                                    update_title = true;
                                }
                            }
                        }

                        send_update(LauncherUpdate::Focus(app_id.clone(), focused)).await?;

                        if update_title {
                            send_update(LauncherUpdate::Title(app_id, window.id, window.title))
                                .await?;
                        }
                    }
                    ToplevelChange::Title(title) => {
                        if let Some(item) = lock!(items).get_mut(&app_id) {
                            item.set_window_name(window.id, title.clone());
                        }

                        send_update(LauncherUpdate::Title(app_id, window.id, title)).await?;
                    }
                    ToplevelChange::Fullscreen(_) => {}
                }
            }

            Ok::<(), mpsc::error::SendError<ModuleUpdateEvent<LauncherUpdate>>>(())
        });

        // listen to ui events
        spawn(async move {
            while let Some(event) = rx.recv().await {
                trace!("{:?}", event);

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
                    let wl = wayland::get_client().await;
                    let items = lock!(items);

                    let id = match event {
                        ItemEvent::FocusItem(app_id) => items
                            .get(&app_id)
                            .and_then(|item| item.windows.first().map(|(_, win)| win.id)),
                        ItemEvent::FocusWindow(id) => Some(id),
                        ItemEvent::OpenItem(_) => unreachable!(),
                    };

                    if let Some(id) = id {
                        let toplevels = read_lock!(wl.toplevels);
                        let seat = wl.seats.first().expect("Failed to get Wayland seat");
                        if let Some((_top, handle)) = toplevels.get(&id) {
                            handle.activate(seat);
                        };
                    }
                }
            }

            Ok::<(), swayipc_async::Error>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> crate::Result<ModuleWidget<gtk::Box>> {
        let icon_theme = IconTheme::new();
        if let Some(ref theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(theme));
        }

        let container = gtk::Box::new(info.bar_position.get_orientation(), 0);

        {
            let container = container.clone();

            let show_names = self.show_names;
            let show_icons = self.show_icons;
            let orientation = info.bar_position.get_orientation();

            let mut buttons = IndexMap::<String, ItemButton>::new();

            let controller_tx2 = context.controller_tx.clone();
            context.widget_rx.attach(None, move |event| {
                match event {
                    LauncherUpdate::AddItem(item) => {
                        debug!("Adding item with id {}", item.app_id);

                        if let Some(button) = buttons.get(&item.app_id) {
                            button.set_open(true);
                        } else {
                            let button = ItemButton::new(
                                &item,
                                show_names,
                                show_icons,
                                orientation,
                                &icon_theme,
                                &context.tx,
                                &controller_tx2,
                            );

                            container.add(&button.button);
                            buttons.insert(item.app_id, button);
                        }
                    }
                    LauncherUpdate::AddWindow(app_id, _) => {
                        if let Some(button) = buttons.get(&app_id) {
                            button.set_open(true);

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
                    LauncherUpdate::RemoveWindow(app_id, _) => {
                        if let Some(button) = buttons.get(&app_id) {
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

                Continue(true)
            });
        }

        let popup = self.into_popup(context.controller_tx, context.popup_rx);
        Ok(ModuleWidget {
            widget: container,
            popup,
        })
    }

    fn into_popup(
        self,
        controller_tx: Sender<Self::ReceiveMessage>,
        rx: glib::Receiver<Self::SendMessage>,
    ) -> Option<gtk::Box> {
        const MAX_WIDTH: i32 = 250;

        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .name("popup-launcher")
            .build();

        // we need some content to force the container to have a size
        let placeholder = Button::with_label("PLACEHOLDER");
        placeholder.set_width_request(MAX_WIDTH);
        container.add(&placeholder);

        let mut buttons = IndexMap::<String, IndexMap<usize, Button>>::new();

        {
            let container = container.clone();
            rx.attach(None, move |event| {
                match event {
                    LauncherUpdate::AddItem(item) => {
                        let app_id = item.app_id.clone();
                        trace!("Adding item with id '{app_id}' to the popup: {item:?}");

                        let window_buttons = item
                            .windows
                            .into_iter()
                            .map(|(_, win)| {
                                let button = Button::builder()
                                    .label(&clamp(&win.name))
                                    .height_request(40)
                                    .build();

                                {
                                    let tx = controller_tx.clone();
                                    button.connect_clicked(move |button| {
                                        try_send!(tx, ItemEvent::FocusWindow(win.id));

                                        if let Some(win) = button.window() {
                                            win.hide();
                                        }
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
                                .label(&clamp(&win.name))
                                .build();

                            {
                                let tx = controller_tx.clone();
                                button.connect_clicked(move |button| {
                                    try_send!(tx, ItemEvent::FocusWindow(win.id));

                                    if let Some(win) = button.window() {
                                        win.hide();
                                    }
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

                Continue(true)
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
fn clamp(str: &str) -> String {
    const MAX_CHARS: usize = 24;

    if str.len() > MAX_CHARS {
        str.chars().take(MAX_CHARS - 3).collect::<String>() + "..."
    } else {
        str.to_string()
    }
}
