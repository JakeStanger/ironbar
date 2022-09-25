mod item;
mod open_state;

use crate::collection::Collection;
use crate::icon::find_desktop_file;
use crate::modules::launcher::item::{Item, ItemButton, Window};
use crate::modules::launcher::open_state::OpenState;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::sway::get_sub_client;
use crate::sway::node::{get_node_id, get_open_windows};
use crate::{await_sync, get_client};
use color_eyre::{Help, Report};
use glib::Continue;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Orientation};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use swayipc_async::WindowChange;
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
    RemoveWindow(String, i64),
    /// Sets title for `app_id`
    Title(String, i64, Option<String>),
    /// Focuses first `app_id`, unfocuses second `app_id` (if present)
    Focus(String, Option<String>),
    /// Marks the item with `app_id` as urgent or not urgent
    Urgent(String, bool),
    /// Declares the item with `app_id` has been hovered over
    Hover(String),
}

#[derive(Debug)]
pub enum ItemEvent {
    FocusItem(String),
    FocusWindow(i64),
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

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> crate::Result<()> {
        let items = match &self.favorites {
            Some(favorites) => favorites
                .iter()
                .map(|app_id| {
                    (
                        app_id.to_string(),
                        Item::new(app_id.to_string(), OpenState::Closed, true),
                    )
                })
                .collect::<Collection<_, _>>(),
            None => Collection::new(),
        };

        let items = Arc::new(Mutex::new(items));

        let open_windows = await_sync(async {
            let sway = get_client().await;
            let mut sway = sway.lock().await;
            get_open_windows(&mut sway).await
        })?;

        {
            let mut items = items.lock().expect("Failed to get lock on items");
            for window in open_windows {
                let id = get_node_id(&window).to_string();

                let item = items.get_mut(&id);
                match item {
                    Some(item) => {
                        item.merge_node(window);
                    }
                    None => {
                        items.insert(id, window.into());
                    }
                }
            }

            let items = items.iter();
            for item in items {
                tx.try_send(ModuleUpdateEvent::Update(LauncherUpdate::AddItem(
                    item.clone(),
                )))?;
            }
        }

        let items2 = Arc::clone(&items);
        spawn(async move {
            let items = items2;

            let mut srx = {
                let sway = get_sub_client();
                sway.subscribe_window()
            };

            while let Ok(event) = srx.recv().await {
                trace!("event: {:?}", event);

                let window = event.container;
                let id = get_node_id(&window).to_string();

                let send_update =
                    |update: LauncherUpdate| tx.send(ModuleUpdateEvent::Update(update));

                let items = || items.lock().expect("Failed to get lock on items");

                match event.change {
                    WindowChange::New => {
                        let new_item = {
                            let mut items = items();
                            match items.get_mut(&id) {
                                None => {
                                    let item: Item = window.into();
                                    items.insert(id.clone(), item.clone());

                                    ItemOrWindow::Item(item)
                                }
                                Some(item) => {
                                    let window = item.merge_node(window);
                                    ItemOrWindow::Window(window)
                                }
                            }
                        };

                        match new_item {
                            ItemOrWindow::Item(item) => {
                                send_update(LauncherUpdate::AddItem(item)).await
                            }
                            ItemOrWindow::Window(window) => {
                                send_update(LauncherUpdate::AddWindow(id, window)).await
                            }
                        }?;
                    }
                    WindowChange::Close => {
                        let remove_item = {
                            let mut items = items();
                            match items.get_mut(&id) {
                                Some(item) => {
                                    item.unmerge_node(&window);

                                    if item.windows.is_empty() {
                                        items.remove(&id);
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
                                send_update(LauncherUpdate::RemoveItem(id)).await?;
                            }
                            Some(ItemOrWindowId::Window) => {
                                send_update(LauncherUpdate::RemoveWindow(id, window.id)).await?;
                            }
                            None => {}
                        };
                    }
                    WindowChange::Focus => {
                        let prev_id = {
                            let mut items = items();

                            let prev_focused =
                                items.iter_mut().find(|item| item.open_state.is_focused());
                            if let Some(prev_focused) = prev_focused {
                                prev_focused.set_unfocused();
                                Some(prev_focused.app_id.to_string())
                            } else {
                                None
                            }
                        };

                        let mut update_title = false;
                        if let Some(item) = items().get_mut(&id) {
                            item.set_window_focused(window.id, true);

                            // might be switching focus between windows of same app
                            if item.windows.len() > 1 {
                                item.set_window_name(window.id, window.name.clone());
                                update_title = true;
                            }
                        }

                        send_update(LauncherUpdate::Focus(id.clone(), prev_id)).await?;

                        if update_title {
                            send_update(LauncherUpdate::Title(id, window.id, window.name)).await?;
                        }
                    }
                    WindowChange::Title => {
                        if let Some(item) = items().get_mut(&id) {
                            item.set_window_name(window.id, window.name.clone());
                        }

                        send_update(LauncherUpdate::Title(id, window.id, window.name)).await?;
                    }
                    WindowChange::Urgent => {
                        if let Some(item) = items().get_mut(&id) {
                            item.set_window_urgent(window.id, window.urgent);
                        }

                        send_update(LauncherUpdate::Urgent(id, window.urgent)).await?;
                    }
                    _ => {}
                }
            }

            Ok::<(), mpsc::error::SendError<ModuleUpdateEvent<LauncherUpdate>>>(())
        });

        // listen to ui events
        spawn(async move {
            let sway = get_client().await;

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
                    let selector = {
                        let items = items.lock().expect("Failed to get lock on items");

                        match event {
                            ItemEvent::FocusItem(app_id) => items.get(&app_id).map(|item| {
                                if item.is_xwayland {
                                    format!("[class={}]", app_id)
                                } else {
                                    format!("[app_id={}]", app_id)
                                }
                            }),
                            ItemEvent::FocusWindow(con_id) => Some(format!("[con_id={}]", con_id)),
                            ItemEvent::OpenItem(_) => unreachable!(),
                        }
                    };

                    if let Some(selector) = selector {
                        let mut sway = sway.lock().await;
                        sway.run_command(format!("{} focus", selector)).await?;
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
        _info: &ModuleInfo,
    ) -> crate::Result<ModuleWidget<gtk::Box>> {
        let icon_theme = IconTheme::new();
        if let Some(ref theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(theme));
        }

        let container = gtk::Box::new(Orientation::Horizontal, 0);

        {
            let container = container.clone();

            let show_names = self.show_names;
            let show_icons = self.show_icons;

            let mut buttons = Collection::<String, ItemButton>::new();

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
                            let mut menu_state = button
                                .menu_state
                                .write()
                                .expect("Failed to get write lock on item menu state");
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
                            let mut menu_state = button
                                .menu_state
                                .write()
                                .expect("Failed to get write lock on item menu state");
                            menu_state.num_windows -= 1;
                        }
                    }
                    LauncherUpdate::Focus(new, prev) => {
                        debug!(
                            "Changing focus to item with id {} (removing from {:?})",
                            new, prev
                        );

                        if let Some(prev) = prev {
                            if let Some(button) = buttons.get(&prev) {
                                button.set_focused(false);
                            }
                        }

                        if let Some(button) = buttons.get(&new) {
                            button.set_focused(true);
                        }
                    }
                    LauncherUpdate::Title(app_id, _, name) => {
                        debug!("Updating title for item with id {}: {:?}", app_id, name);

                        if show_names {
                            if let Some(button) = buttons.get(&app_id) {
                                button.button.set_label(&name.unwrap_or_default());
                            }
                        }
                    }
                    LauncherUpdate::Urgent(app_id, urgent) => {
                        debug!("Updating urgency for item with id {}: {}", app_id, urgent);

                        if let Some(button) = buttons.get(&app_id) {
                            button.set_urgent(urgent);
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
        let container = gtk::Box::new(Orientation::Vertical, 0);

        let mut buttons = Collection::<String, Collection<i64, Button>>::new();

        {
            let container = container.clone();
            rx.attach(None, move |event| {
                match event {
                    LauncherUpdate::AddItem(item) => {
                        let app_id = item.app_id.clone();

                        let window_buttons = item
                            .windows
                            .into_iter()
                            .map(|win| {
                                let button = Button::builder()
                                    .label(win.name.as_ref().unwrap_or(&String::new()))
                                    .height_request(40)
                                    .width_request(100)
                                    .build();

                                {
                                    let tx = controller_tx.clone();
                                    button.connect_clicked(move |button| {
                                        tx.try_send(ItemEvent::FocusWindow(win.id))
                                            .expect("Failed to send window click event");

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
                        if let Some(buttons) = buttons.get_mut(&app_id) {
                            let button = Button::builder()
                                .label(win.name.as_ref().unwrap_or(&String::new()))
                                .height_request(40)
                                .width_request(100)
                                .build();

                            {
                                let tx = controller_tx.clone();
                                button.connect_clicked(move |button| {
                                    tx.try_send(ItemEvent::FocusWindow(win.id))
                                        .expect("Failed to send window click event");

                                    if let Some(win) = button.window() {
                                        win.hide();
                                    }
                                });
                            }

                            buttons.insert(win.id, button);
                        }
                    }
                    LauncherUpdate::RemoveWindow(app_id, win_id) => {
                        if let Some(buttons) = buttons.get_mut(&app_id) {
                            buttons.remove(&win_id);
                        }
                    }
                    LauncherUpdate::Title(app_id, win_id, title) => {
                        if let Some(buttons) = buttons.get_mut(&app_id) {
                            if let Some(button) = buttons.get(&win_id) {
                                if let Some(title) = title {
                                    button.set_label(&title);
                                }
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
                            for button in buttons {
                                container.add(button);
                            }

                            container.show_all();
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
