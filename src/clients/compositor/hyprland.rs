#[cfg(feature = "bindmode+hyprland")]
use super::{BindModeClient, BindModeUpdate};
#[cfg(feature = "keyboard+hyprland")]
use super::{KeyboardLayoutClient, KeyboardLayoutUpdate};
use super::{Visibility, Workspace};
use crate::channels::SyncSenderExt;
use crate::{arc_mut, lock, spawn_blocking};
use color_eyre::Result;
use hyprland::ctl::switch_xkb_layout;
use hyprland::data::{Devices, Workspace as HWorkspace, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::event_listener::EventListener;
use hyprland::prelude::*;
use hyprland::shared::{HyprDataVec, WorkspaceType};
use tokio::sync::broadcast::{Receiver, Sender, channel};
use tracing::{debug, error, info, warn};

#[cfg(feature = "workspaces")]
use super::WorkspaceUpdate;

#[derive(Debug)]
struct TxRx<T> {
    tx: Sender<T>,
    _rx: Receiver<T>,
}
impl<T: Clone> TxRx<T> {
    fn new() -> Self {
        let (tx, rx) = channel(16);
        Self { tx, _rx: rx }
    }
}

#[derive(Debug)]
pub struct Client {
    #[cfg(feature = "workspaces+hyprland")]
    workspace: TxRx<WorkspaceUpdate>,

    #[cfg(feature = "keyboard+hyprland")]
    keyboard_layout: TxRx<KeyboardLayoutUpdate>,

    #[cfg(feature = "bindmode+hyprland")]
    bindmode: TxRx<BindModeUpdate>,
}

impl Client {
    pub(crate) fn new() -> Self {
        let instance = Self {
            #[cfg(feature = "workspaces+hyprland")]
            workspace: TxRx::new(),
            #[cfg(feature = "keyboard+hyprland")]
            keyboard_layout: TxRx::new(),
            #[cfg(feature = "bindmode+hyprland")]
            bindmode: TxRx::new(),
        };

        instance.listen_events();
        instance
    }

    fn listen_events(&self) {
        info!("Starting Hyprland event listener");

        #[cfg(feature = "workspaces+hyprland")]
        let workspace_tx = self.workspace.tx.clone();

        #[cfg(feature = "keyboard+hyprland")]
        let keyboard_layout_tx = self.keyboard_layout.tx.clone();

        #[cfg(feature = "bindmode+hyprland")]
        let bindmode_tx = self.bindmode.tx.clone();

        spawn_blocking(move || {
            let mut event_listener = EventListener::new();

            // we need a lock to ensure events don't run at the same time
            let lock = arc_mut!(());

            // cache the active workspace since Hyprland doesn't give us the prev active
            #[cfg(feature = "workspaces+hyprland")]
            Self::listen_workspace_events(&workspace_tx, &mut event_listener, &lock);

            #[cfg(feature = "keyboard+hyprland")]
            Self::listen_keyboard_events(&keyboard_layout_tx, &mut event_listener, &lock);

            #[cfg(feature = "bindmode+hyprland")]
            Self::listen_bindmode_events(&bindmode_tx, &mut event_listener, &lock);

            if let Err(err) = event_listener.start_listener() {
                error!("Failed to start listener: {err:#}");
            }
        });
    }

    #[cfg(feature = "workspaces+hyprland")]
    fn listen_workspace_events(
        tx: &Sender<WorkspaceUpdate>,
        event_listener: &mut EventListener,
        lock: &std::sync::Arc<std::sync::Mutex<()>>,
    ) {
        let active = Self::get_active_workspace().map_or_else(
            |err| {
                error!("Failed to get active workspace: {err:#?}");
                None
            },
            Some,
        );
        let active = arc_mut!(active);

        {
            let tx = tx.clone();
            let lock = lock.clone();
            let active = active.clone();

            event_listener.add_workspace_added_handler(move |event| {
                let _lock = lock!(lock);
                debug!("Added workspace: {event:?}");

                let workspace_name = get_workspace_name(event.name);
                let prev_workspace = lock!(active);

                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                match workspace {
                    Ok(Some(workspace)) => {
                        tx.send_expect(WorkspaceUpdate::Add(workspace));
                    }
                    Err(e) => error!("Failed to get workspace: {e:#}"),
                    _ => {}
                }
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();
            let active = active.clone();

            event_listener.add_workspace_changed_handler(move |event| {
                let _lock = lock!(lock);

                let mut prev_workspace = lock!(active);

                debug!(
                    "Received workspace change: {:?} -> {event:?}",
                    prev_workspace.as_ref().map(|w| &w.id)
                );

                let workspace_name = get_workspace_name(event.name);
                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                match workspace {
                    Ok(Some(workspace)) if !workspace.visibility.is_focused() => {
                        Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                    }
                    Ok(None) => {
                        error!("Unable to locate workspace");
                    }
                    Err(e) => error!("Failed to get workspace: {e:#}"),
                    _ => {}
                }
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();
            let active = active.clone();

            event_listener.add_active_monitor_changed_handler(move |event_data| {
                let _lock = lock!(lock);
                let Some(workspace_type) = event_data.workspace_name else {
                    warn!("Received active monitor change with no workspace name");
                    return;
                };

                let mut prev_workspace = lock!(active);

                debug!(
                    "Received active monitor change: {:?} -> {workspace_type:?}",
                    prev_workspace.as_ref().map(|w| &w.name)
                );

                let workspace_name = get_workspace_name(workspace_type);
                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                match workspace {
                    Ok(Some(workspace)) if !workspace.visibility.is_focused() => {
                        Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                    }
                    Ok(None) => {
                        error!("Unable to locate workspace");
                    }
                    Err(e) => error!("Failed to get workspace: {e:#}"),
                    _ => {}
                }
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_workspace_moved_handler(move |event_data| {
                let _lock = lock!(lock);
                let workspace_type = event_data.name;
                debug!("Received workspace move: {workspace_type:?}");

                let mut prev_workspace = lock!(active);

                let workspace_name = get_workspace_name(workspace_type);
                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                match workspace {
                    Ok(Some(workspace)) if !workspace.visibility.is_focused() => {
                        Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                    }
                    Ok(None) => {
                        error!("Unable to locate workspace");
                    }
                    Err(e) => error!("Failed to get workspace: {e:#}"),
                    _ => {}
                }
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_workspace_renamed_handler(move |data| {
                let _lock = lock!(lock);
                debug!("Received workspace rename: {data:?}");

                tx.send_expect(WorkspaceUpdate::Rename {
                    id: data.id as i64,
                    name: data.name,
                });
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_workspace_deleted_handler(move |data| {
                let _lock = lock!(lock);
                debug!("Received workspace destroy: {data:?}");
                tx.send_expect(WorkspaceUpdate::Remove(data.id as i64));
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_urgent_state_changed_handler(move |address| {
                let _lock = lock!(lock);
                debug!("Received urgent state: {address:?}");

                let clients = match hyprland::data::Clients::get() {
                    Ok(clients) => clients,
                    Err(err) => {
                        error!("Failed to get clients: {err}");
                        return;
                    }
                };
                clients.iter().find(|c| c.address == address).map_or_else(
                    || {
                        error!("Unable to locate client");
                    },
                    |c| {
                        tx.send_expect(WorkspaceUpdate::Urgent {
                            id: c.workspace.id as i64,
                            urgent: true,
                        });
                    },
                );
            });
        }
    }

    #[cfg(feature = "keyboard+hyprland")]
    fn listen_keyboard_events(
        keyboard_layout_tx: &Sender<KeyboardLayoutUpdate>,
        event_listener: &mut EventListener,
        lock: &std::sync::Arc<std::sync::Mutex<()>>,
    ) {
        let tx = keyboard_layout_tx.clone();
        let lock = lock.clone();

        event_listener.add_layout_changed_handler(move |layout_event| {
            let _lock = lock!(lock);

            let layout = if layout_event.layout_name.is_empty() {
                // FIXME: This field is empty due to bug in `hyprland-rs_0.4.0-alpha.3`. Which is already fixed in last betas

                // The layout may be empty due to a bug in `hyprland-rs`, because of which the `layout_event` is incorrect.
                //
                // Instead of:
                // ```
                // LayoutEvent {
                //     keyboard_name: "keychron-keychron-c2",
                //     layout_name: "English (US)",
                // }
                // ```
                //
                // We get:
                // ```
                // LayoutEvent {
                //     keyboard_name: "keychron-keychron-c2,English (US)",
                //     layout_name: "",
                // }
                // ```
                // 
                // Here we are trying to recover `layout_name` from `keyboard_name`

                let layout = layout_event.keyboard_name.as_str().split(',').nth(1);
                let Some(layout) = layout else {
                    error!(
                        "Failed to get layout from string: {}. The failed logic is a workaround for a bug in `hyprland 0.4.0-alpha.3`", layout_event.keyboard_name);
                    return;
                };

                layout.into()
            }
            else {
                layout_event.layout_name
            };

            debug!("Received layout: {layout:?}");
            tx.send_expect(KeyboardLayoutUpdate(layout));
        });
    }

    #[cfg(feature = "bindmode+hyprland")]
    fn listen_bindmode_events(
        bindmode_tx: &Sender<BindModeUpdate>,
        event_listener: &mut EventListener,
        lock: &std::sync::Arc<std::sync::Mutex<()>>,
    ) {
        let tx = bindmode_tx.clone();
        let lock = lock.clone();

        event_listener.add_sub_map_changed_handler(move |bind_mode| {
            let _lock = lock!(lock);
            debug!("Received bind mode: {bind_mode:?}");

            tx.send_expect(BindModeUpdate {
                name: bind_mode,
                pango_markup: false,
            });
        });
    }

    /// Sends a `WorkspaceUpdate::Focus` event
    /// and updates the active workspace cache.
    #[cfg(feature = "workspaces+hyprland")]
    fn send_focus_change(
        prev_workspace: &mut Option<Workspace>,
        workspace: Workspace,
        tx: &Sender<WorkspaceUpdate>,
    ) {
        tx.send_expect(WorkspaceUpdate::Focus {
            old: prev_workspace.take(),
            new: workspace.clone(),
        });

        tx.send_expect(WorkspaceUpdate::Urgent {
            id: workspace.id,
            urgent: false,
        });

        prev_workspace.replace(workspace);
    }

    /// Gets a workspace by name from the server, given the active workspace if known.
    #[cfg(feature = "workspaces+hyprland")]
    fn get_workspace(name: &str, active: Option<&Workspace>) -> Result<Option<Workspace>> {
        let workspace = Workspaces::get()?.into_iter().find_map(|w| {
            if w.name == name {
                let vis = Visibility::from((&w, active.map(|w| w.name.as_ref()), &|w| {
                    create_is_visible()(w)
                }));

                Some(Workspace::from((vis, w)))
            } else {
                None
            }
        });

        Ok(workspace)
    }

    /// Gets the active workspace from the server.
    fn get_active_workspace() -> Result<Workspace> {
        let w = HWorkspace::get_active().map(|w| Workspace::from((Visibility::focused(), w)))?;
        Ok(w)
    }
}

#[cfg(feature = "workspaces+hyprland")]
impl super::WorkspaceClient for Client {
    fn focus(&self, id: i64) {
        let identifier = WorkspaceIdentifierWithSpecial::Id(id as i32);

        if let Err(e) = Dispatch::call(DispatchType::Workspace(identifier)) {
            error!("Couldn't focus workspace '{id}': {e:#}");
        }
    }

    fn subscribe(&self) -> Receiver<WorkspaceUpdate> {
        let rx = self.workspace.tx.subscribe();

        let active_id = HWorkspace::get_active().ok().map(|active| active.name);
        let is_visible = create_is_visible();

        match Workspaces::get() {
            Ok(workspaces) => {
                let workspaces = workspaces
                    .into_iter()
                    .map(|w| {
                        let vis = Visibility::from((&w, active_id.as_deref(), &is_visible));
                        Workspace::from((vis, w))
                    })
                    .collect();

                self.workspace
                    .tx
                    .send_expect(WorkspaceUpdate::Init(workspaces));
            }
            Err(e) => {
                error!("Failed to get workspaces: {e:#}");
            }
        }

        rx
    }
}

#[cfg(feature = "keyboard+hyprland")]
impl KeyboardLayoutClient for Client {
    fn set_next_active(&self) {
        let Ok(devices) = Devices::get() else {
            error!("Failed to get devices");
            return;
        };

        let device = devices
            .keyboards
            .iter()
            .find(|k| k.main)
            .map(|k| k.name.clone());

        if let Some(device) = device {
            if let Err(e) =
                switch_xkb_layout::call(device, switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next)
            {
                error!("Failed to switch keyboard layout due to Hyprland error: {e}");
            }
        } else {
            error!("Failed to get keyboard device from hyprland");
        }
    }

    fn subscribe(&self) -> Receiver<KeyboardLayoutUpdate> {
        let rx = self.keyboard_layout.tx.subscribe();

        match Devices::get().map(|devices| {
            devices
                .keyboards
                .iter()
                .find(|k| k.main)
                .map(|k| k.active_keymap.clone())
        }) {
            Ok(Some(layout)) => {
                self.keyboard_layout
                    .tx
                    .send_expect(KeyboardLayoutUpdate(layout));
            }
            Ok(None) => error!("Failed to get current keyboard layout hyprland"),
            Err(err) => error!("Failed to get devices: {err:#?}"),
        }

        rx
    }
}

#[cfg(feature = "bindmode+hyprland")]
impl BindModeClient for Client {
    fn subscribe(&self) -> Result<Receiver<BindModeUpdate>> {
        Ok(self.bindmode.tx.subscribe())
    }
}

fn get_workspace_name(name: WorkspaceType) -> String {
    match name {
        WorkspaceType::Regular(name) => name,
        WorkspaceType::Special(name) => name.unwrap_or_default(),
    }
}

/// Creates a function which determines if a workspace is visible.
///
/// This function makes a Hyprland call that allocates so it should be cached when possible,
/// but it is only valid so long as workspaces do not change so it should not be stored long term
fn create_is_visible() -> impl Fn(&HWorkspace) -> bool {
    let monitors = hyprland::data::Monitors::get().map_or(Vec::new(), HyprDataVec::to_vec);

    move |w| monitors.iter().any(|m| m.active_workspace.id == w.id)
}

impl From<(Visibility, HWorkspace)> for Workspace {
    fn from((visibility, workspace): (Visibility, HWorkspace)) -> Self {
        Self {
            id: workspace.id as i64,
            name: workspace.name,
            monitor: workspace.monitor,
            visibility,
        }
    }
}

impl<'a, 'f, F> From<(&'a HWorkspace, Option<&str>, F)> for Visibility
where
    F: FnOnce(&'f HWorkspace) -> bool,
    'a: 'f,
{
    fn from((workspace, active_name, is_visible): (&'a HWorkspace, Option<&str>, F)) -> Self {
        if Some(workspace.name.as_str()) == active_name {
            Self::focused()
        } else if is_visible(workspace) {
            Self::visible()
        } else {
            Self::Hidden
        }
    }
}
