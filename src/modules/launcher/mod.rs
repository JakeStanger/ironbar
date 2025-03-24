mod item;
mod open_state;
mod pagination;

use self::item::{AppearanceOptions, Item, ItemButton, Window};
use self::open_state::OpenState;
use super::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::wayland::{self, ToplevelEvent};
use crate::desktop_file::open_program;
use crate::config::{
    CommonConfig, EllipsizeMode, LayoutConfig, TruncateMode, default_launch_command,
};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::modules::launcher::item::ImageTextButton;
use crate::modules::launcher::pagination::{IconContext, Pagination};
use crate::{
    arc_mut, glib_recv, lock, module_impl, rc_mut, send_async, spawn, try_send, write_lock,
};
use color_eyre::{Help, Report};
use gtk::prelude::*;
use gtk::{Button, Orientation};
use indexmap::IndexMap;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, trace, warn};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LauncherModule {
    /// List of app IDs (or classes) to always show regardless of open state,
    /// in the order specified.
    ///
    /// **Default**: `null`
    favorites: Option<Vec<String>>,

    /// Whether to show application names on the bar.
    ///
    /// **Default**: `false`
    #[serde(default = "crate::config::default_false")]
    show_names: bool,

    /// Whether to show application icons on the bar.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_icons: bool,

    /// Size in pixels to render icon at (image icons only).
    ///
    /// **Default**: `32`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    /// Whether items should be added from right-to-left
    /// instead of left-to-right.
    ///
    /// This includes favourites.
    ///
    /// **Default**: `false`
    #[serde(default = "crate::config::default_false")]
    reversed: bool,

    /// Whether to minimize a window if it is focused when clicked.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    minimize_focused: bool,

    /// The number of items to show on a page.
    ///
    /// When the number of items reaches the page size,
    /// pagination controls appear at the start of the widget
    /// which can be used to move forward/back through the list of items.
    ///
    /// If there are too many to fit, the overflow will be truncated
    /// by the next widget.
    ///
    /// **Default**: `1000`.
    #[serde(default = "default_page_size")]
    page_size: usize,

    /// Module UI icons (separate from app icons shown for items).
    ///
    /// See [icons](#icons).
    #[serde(default)]
    icons: Icons,

    /// Size in pixels to render pagination icons at (image icons only).
    ///
    /// **Default**: `16`
    #[serde(default = "default_icon_size_pagination")]
    pagination_icon_size: i32,

    // -- common --
    /// Truncate application names on the bar if they get too long.
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `Auto (end)`
    #[serde(default)]
    truncate: TruncateMode,

    /// Truncate application names in popups if they get too long.
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `{ mode = "middle" max_length = 25 }`
    #[serde(default = "default_truncate_popup")]
    truncate_popup: TruncateMode,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// Command used to launch applications.
    ///
    /// **Default**: `gtk-launch`
    #[serde(default = "default_launch_command")]
    launch_command: String,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct Icons {
    /// Icon to show for page back button.
    ///
    /// **Default**: `󰅁`
    #[serde(default = "default_icon_page_back")]
    page_back: String,

    /// Icon to show for page back button.
    ///
    /// **Default**: `>`
    #[serde(default = "default_icon_page_forward")]
    page_forward: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            page_back: default_icon_page_back(),
            page_forward: default_icon_page_forward(),
        }
    }
}

const fn default_icon_size() -> i32 {
    32
}

const fn default_icon_size_pagination() -> i32 {
    default_icon_size() / 2
}

const fn default_page_size() -> usize {
    1000
}

fn default_icon_page_back() -> String {
    String::from("󰅁")
}

fn default_icon_page_forward() -> String {
    String::from("󰅂")
}

const fn default_truncate_popup() -> TruncateMode {
    TruncateMode::Length {
        mode: EllipsizeMode::Middle,
        length: None,
        max_length: Some(25),
    }
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
    MinimizeItem(String),
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

    module_impl!("launcher");

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
        let desktop_files = context.ironbar.desktop_files();
        spawn(async move {
            let items = items2;
            let tx = tx2;

            // Build app_id mapping once at startup
            let mut app_id_map = IndexMap::<String, String>::new();
            {
                let favorites: Vec<_> = lock!(items).keys().cloned().collect();
                for fav in favorites {
                    if let Ok(Some(file)) = desktop_files.find(&fav).await
                        && let Some(wm_class) = file.startup_wm_class
                    {
                        app_id_map.insert(wm_class, fav);
                    }
                }
            }

            let resolve_app_id = |app_id: &str| {
                app_id_map
                    .get(app_id)
                    .cloned()
                    .unwrap_or_else(|| app_id.to_string())
            };

            let mut wlrx = wl.subscribe_toplevels();
            let handles = wl.toplevel_info_all();

            for info in handles {
                let mut items = lock!(items);
                let app_id = resolve_app_id(&info.app_id);
                if let Some(item) = items.get_mut(&app_id) {
                    item.merge_toplevel(info.clone());
                } else {
                    let mut item = Item::from(info.clone());
                    item.app_id = app_id.clone();
                    items.insert(app_id, item);
                }
            }

            {
                let items = {
                    let items = lock!(items);

                    items
                        .iter()
                        .map(|(_, item)| item.clone())
                        .collect::<Vec<_>>() // need to collect to be able to drop lock
                };

                for item in items {
                    tx.send_update(LauncherUpdate::AddItem(item)).await;
                }
            }

            let send_update = |update: LauncherUpdate| tx.send(ModuleUpdateEvent::Update(update));

            while let Ok(event) = wlrx.recv().await {
                trace!("event: {:?}", event);

                match event {
                    ToplevelEvent::New(info) => {
                        let app_id = resolve_app_id(&info.app_id);

                        let new_item = {
                            let mut items = lock!(items);
                            match items.get_mut(&app_id) {
                                None => {
                                    let mut item: Item = info.into();
                                    item.app_id = app_id.clone();
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
                        let app_id = resolve_app_id(&info.app_id);
                        // check if open, as updates can be sent as program closes
                        // if it's a focused favourite closing, it otherwise incorrectly re-focuses.
                        let is_open = if let Some(item) = lock!(items).get_mut(&app_id) {
                            item.set_window_focused(info.id, info.focused);
                            item.set_window_name(info.id, info.title.clone());

                            item.open_state.is_open()
                        } else {
                            false
                        };

                        send_update(LauncherUpdate::Focus(
                            app_id.clone(),
                            is_open && info.focused,
                        ))
                        .await?;
                        send_update(LauncherUpdate::Title(
                            app_id.clone(),
                            info.id,
                            info.title.clone(),
                        ))
                        .await?;
                    }
                    ToplevelEvent::Remove(info) => {
                        let app_id = resolve_app_id(&info.app_id);
                        let remove_item = {
                            let mut items = lock!(items);
                            match items.get_mut(&app_id) {
                                Some(item) => {
                                    item.unmerge_toplevel(&info);

                                    if item.windows.is_empty() {
                                        items.shift_remove(&app_id);
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
                                send_update(LauncherUpdate::RemoveItem(app_id.clone())).await?;
                            }
                            Some(ItemOrWindowId::Window) => {
                                send_update(LauncherUpdate::RemoveWindow(app_id.clone(), info.id))
                                    .await?;
                            }
                            None => {}
                        }
                    }
                }
            }

            Ok::<(), Report>(())
        });

        // listen to ui events
        let minimize_focused = self.minimize_focused;
        let wl = context.client::<wayland::Client>();

        let desktop_files = context.ironbar.desktop_files();
        let launch_command_str: String = self.launch_command.clone();

        spawn(async move {
            while let Some(event) = rx.recv().await {
                if let ItemEvent::OpenItem(app_id) = event {
                    match desktop_files.find(&app_id).await {
                        Ok(Some(file)) => {
                            open_program(&file.file_name, &launch_command_str).await;
                        }
                        Ok(None) => warn!("Could not find applications file for {}", app_id),
                        Err(err) => error!("Failed to find parse file for {}: {}", app_id, err),
                    }
                } else {
                    tx.send_expect(ModuleUpdateEvent::ClosePopup).await;

                    let minimize_window = matches!(event, ItemEvent::MinimizeItem(_));

                    let id = match event {
                        ItemEvent::FocusItem(app_id) | ItemEvent::MinimizeItem(app_id) => {
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

                    if let Some(id) = id
                        && let Some(window) = lock!(items)
                            .iter()
                            .find_map(|(_, item)| item.windows.get(&id))
                    {
                        debug!("Focusing window {id}: {}", window.name);
                        if minimize_window && minimize_focused {
                            wl.toplevel_minimize(window.id);
                        } else {
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
        let container = gtk::Box::new(self.layout.orientation(info), 0);
        let page_size = self.page_size;

        let image_provider = context.ironbar.image_provider();
        let mut buttons = rc_mut!(IndexMap::<String, ItemButton>::new());

        let pagination = Pagination::new(
            &container,
            self.page_size,
            self.layout.orientation(info),
            &IconContext {
                back: &self.icons.page_back,
                fwd: &self.icons.page_forward,
                size: self.pagination_icon_size,
            },
            &image_provider,
        );

        {
            let appearance_options = AppearanceOptions {
                show_names: self.show_names,
                show_icons: self.show_icons,
                icon_size: self.icon_size,
                truncate: self.truncate,
                orientation: self.layout.orientation(info),
                angle: self.layout.angle(info),
                justify: self.layout.justify.into(),
            };

            let show_names = self.show_names;
            let bar_position = info.bar_position;

            let buttons = buttons.clone();

            let rx = context.subscribe();

            rx.recv_glib(
                (&container, &context.controller_tx, &context.tx),
                move |(container, controller_tx, tx), event: LauncherUpdate| {
                    // all widgets show by default
                    // so check if pagination should be shown
                    // to ensure correct state on init.
                    if buttons.borrow().len() <= page_size {
                        pagination.hide();
                    }

                    match event {
                        LauncherUpdate::AddItem(item) => {
                            debug!("Adding item with id '{}' to the bar: {item:?}", item.app_id);

                            if let Some(button) = buttons.borrow().get(&item.app_id) {
                                button.set_open(true);
                                button.set_focused(item.open_state.is_focused());
                            } else {
                                let button = ItemButton::new(
                                    &item,
                                    appearance_options,
                                    image_provider.clone(),
                                    bar_position,
                                    tx,
                                    controller_tx,
                                );

                                if self.reversed {
                                    container.prepend(button.button.deref());
                                } else {
                                    container.append(button.button.deref());
                                }

                                if buttons.borrow().len() + 1 >= pagination.offset() + page_size {
                                    button.button.set_visible(false);
                                    pagination.set_sensitive_fwd(true);
                                }

                                if buttons.borrow().len() + 1 > page_size {
                                    pagination.show();
                                }

                                buttons.borrow_mut().insert(item.app_id, button);
                            }
                        }
                        LauncherUpdate::AddWindow(app_id, win) => {
                            if let Some(button) = buttons.borrow().get(&app_id) {
                                button.set_open(true);
                                button.set_focused(win.open_state.is_focused());

                                write_lock!(button.menu_state).num_windows += 1;
                            }
                        }
                        LauncherUpdate::RemoveItem(app_id) => {
                            debug!("Removing item with id {}", app_id);

                            if let Some(button) = buttons.borrow().get(&app_id) {
                                if button.persistent {
                                    button.set_open(false);
                                    if button.show_names {
                                        button.button.label.set_label(&app_id);
                                    }
                                } else {
                                    container.remove(&button.button.button);
                                    buttons.borrow_mut().shift_remove(&app_id);
                                }
                            }

                            if buttons.borrow().len() < pagination.offset() + page_size {
                                pagination.set_sensitive_fwd(false);
                            }

                            if buttons.borrow().len() <= page_size {
                                pagination.hide();
                            }
                        }
                        LauncherUpdate::RemoveWindow(app_id, win_id) => {
                            debug!("Removing window {win_id} with id {app_id}");

                            if let Some(button) = buttons.borrow().get(&app_id) {
                                button.set_focused(false);

                                let mut menu_state = write_lock!(button.menu_state);
                                menu_state.num_windows -= 1;
                            }
                        }
                        LauncherUpdate::Focus(app_id, focus) => {
                            debug!("Changing focus to {} on item with id {}", focus, app_id);

                            if let Some(button) = buttons.borrow().get(&app_id) {
                                button.set_focused(focus);
                            }
                        }
                        LauncherUpdate::Title(app_id, _, name) => {
                            debug!("Updating title for item with id {}: {:?}", app_id, name);

                            if show_names && let Some(button) = buttons.borrow().get(&app_id) {
                                button.button.label.set_label(&name);
                            }
                        }
                        LauncherUpdate::Hover(_) => {}
                    }
                },
            );
        }

        let popup = self.into_popup(context, info).into_popup_parts(vec![]); // since item buttons are dynamic, they pass their geometry directly

        Ok(ModuleParts {
            widget: container,
            popup,
        })
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        const MAX_WIDTH: i32 = 250;

        let container = gtk::Box::new(Orientation::Vertical, 0);

        // we need some content to force the container to have a size
        let placeholder = Button::with_label("PLACEHOLDER");
        placeholder.set_width_request(MAX_WIDTH);
        container.append(&placeholder);

        let mut buttons = IndexMap::<String, IndexMap<usize, ImageTextButton>>::new();

        context
            .subscribe()
            .recv_glib(&container, move |container, event| {
                match event {
                    LauncherUpdate::AddItem(item) => {
                        let app_id = item.app_id.clone();
                        trace!("Adding item with id '{app_id}' to the popup: {item:?}");

                        let window_buttons = item
                            .windows
                            .into_iter()
                            .map(|(_, win)| {
                                // TODO: Currently has a useless image
                                let button = ImageTextButton::new(Orientation::Horizontal);
                                button.set_height_request(40);
                                button.label.set_label(&win.name);
                                button.label.truncate(self.truncate_popup);

                                {
                                    let tx = context.controller_tx.clone();
                                    button.connect_clicked(move |_| {
                                        tx.send_spawn(ItemEvent::FocusWindow(win.id));
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
                            // TODO: Currently has a useless image
                            let button = ImageTextButton::new(Orientation::Horizontal);
                            button.set_height_request(40);
                            button.label.set_label(&win.name);
                            button.label.truncate(self.truncate_popup);

                            {
                                let tx = context.controller_tx.clone();
                                button.connect_clicked(move |_button| {
                                    tx.send_spawn(ItemEvent::FocusWindow(win.id));
                                });
                            }

                            buttons.insert(win.id, button);
                        }
                    }
                    LauncherUpdate::RemoveWindow(app_id, win_id) => {
                        debug!("Removing window from popup for '{app_id}': {win_id}");

                        if let Some(buttons) = buttons.get_mut(&app_id) {
                            buttons.shift_remove(&win_id);
                        }
                    }
                    LauncherUpdate::Title(app_id, win_id, title) => {
                        debug!(
                            "Updating window title on popup for '{app_id}'/{win_id} to '{title}'"
                        );

                        if let Some(buttons) = buttons.get_mut(&app_id)
                            && let Some(button) = buttons.get(&win_id)
                        {
                            button.label.set_label(&title);
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
                                button.add_class("popup-item");
                                container.append(&button.button);
                            }

                            container.show();
                            container.set_width_request(MAX_WIDTH);
                        }
                    }
                    _ => {}
                }
            });

        Some(container)
    }
}
