use super::{Visibility, Workspace, WorkspaceUpdate};
use crate::{arc_mut, lock, send, spawn_blocking};
use color_eyre::Result;
use hyprland::ctl::switch_xkb_layout;
use hyprland::data::{Devices, Workspace as HWorkspace, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::event_listener::EventListener;
use hyprland::prelude::*;
use hyprland::shared::{HyprDataVec, WorkspaceType};
use tokio::sync::broadcast::{Receiver, Sender, channel};
use tracing::{debug, error, info};

#[derive(Debug)]
struct TxRx<T> {
    tx: Sender<T>,
    _rx: Receiver<T>,
}

#[derive(Debug)]
pub struct Client {
    #[cfg(feature = "workspaces+hyprland")]
    workspace: TxRx<WorkspaceUpdate>,

    #[cfg(feature = "keyboard+hyprland")]
    keyboard_layout: TxRx<KeyboardLayoutUpdate>,
}

impl Client {
    pub(crate) fn new() -> Self {
        #[cfg(feature = "workspaces+hyprland")]
        let (workspace_tx, workspace_rx) = channel(16);

        #[cfg(feature = "keyboard+hyprland")]
        let (keyboard_layout_tx, keyboard_layout_rx) = channel(16);

        let instance = Self {
            #[cfg(feature = "workspaces+hyprland")]
            workspace: TxRx {
                tx: workspace_tx,
                _rx: workspace_rx,
            },
            #[cfg(feature = "keyboard+hyprland")]
            keyboard_layout: TxRx {
                tx: keyboard_layout_tx,
                _rx: keyboard_layout_rx,
            },
        };

        instance.listen_events();
        instance
    }

    fn listen_events(&self) {
        info!("Starting Hyprland event listener");

        #[cfg(feature = "workspaces+hyprland")]
        let tx = self.workspace.tx.clone();

        #[cfg(feature = "keyboard+hyprland")]
        let keyboard_layout_tx = self.keyboard_layout.tx.clone();

        spawn_blocking(move || {
            let mut event_listener = EventListener::new();

            // we need a lock to ensure events don't run at the same time
            let lock = arc_mut!(());

            // cache the active workspace since Hyprland doesn't give us the prev active
            #[cfg(feature = "workspaces+hyprland")]
            Self::listen_workspace_events(tx, &mut event_listener, &lock);

            #[cfg(feature = "keyboard+hyprland")]
            Self::listen_keyboard_events(keyboard_layout_tx, &mut event_listener, lock);

            event_listener
                .start_listener()
                .expect("Failed to start listener");
        });
    }

    #[cfg(feature = "workspaces+hyprland")]
    fn listen_workspace_events(
        tx: Sender<WorkspaceUpdate>,
        event_listener: &mut EventListener,
        lock: &std::sync::Arc<std::sync::Mutex<()>>,
    ) {
        let active = Self::get_active_workspace().expect("Failed to get active workspace");
        let active = arc_mut!(Some(active));

        {
            let tx = tx.clone();
            let lock = lock.clone();
            let active = active.clone();

            event_listener.add_workspace_added_handler(move |workspace_type| {
                let _lock = lock!(lock);
                debug!("Added workspace: {workspace_type:?}");

                let workspace_name = get_workspace_name(workspace_type);
                let prev_workspace = lock!(active);

                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                if let Some(workspace) = workspace {
                    send!(tx, WorkspaceUpdate::Add(workspace));
                }
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();
            let active = active.clone();

            event_listener.add_workspace_change_handler(move |workspace_type| {
                let _lock = lock!(lock);

                let mut prev_workspace = lock!(active);

                debug!(
                    "Received workspace change: {:?} -> {workspace_type:?}",
                    prev_workspace.as_ref().map(|w| &w.id)
                );

                let workspace_name = get_workspace_name(workspace_type);
                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                workspace.map_or_else(
                    || {
                        error!("Unable to locate workspace");
                    },
                    |workspace| {
                        // there may be another type of update so dispatch that regardless of focus change
                        if !workspace.visibility.is_focused() {
                            Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                        }
                    },
                );
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();
            let active = active.clone();

            event_listener.add_active_monitor_change_handler(move |event_data| {
                let _lock = lock!(lock);
                let workspace_type = event_data.workspace;

                let mut prev_workspace = lock!(active);

                debug!(
                    "Received active monitor change: {:?} -> {workspace_type:?}",
                    prev_workspace.as_ref().map(|w| &w.name)
                );

                let workspace_name = get_workspace_name(workspace_type);
                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                if let Some((false, workspace)) = workspace.map(|w| (w.visibility.is_focused(), w))
                {
                    Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                } else {
                    error!("unable to locate workspace: {workspace_name}");
                }
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_workspace_moved_handler(move |event_data| {
                let _lock = lock!(lock);
                let workspace_type = event_data.workspace;
                debug!("Received workspace move: {workspace_type:?}");

                let mut prev_workspace = lock!(active);

                let workspace_name = get_workspace_name(workspace_type);
                let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                if let Some(workspace) = workspace {
                    send!(tx, WorkspaceUpdate::Move(workspace.clone()));

                    if !workspace.visibility.is_focused() {
                        Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                    }
                }
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_workspace_rename_handler(move |data| {
                let _lock = lock!(lock);
                debug!("Received workspace rename: {data:?}");

                send!(
                    tx,
                    WorkspaceUpdate::Rename {
                        id: data.workspace_id as i64,
                        name: data.workspace_name
                    }
                );
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_workspace_destroy_handler(move |data| {
                let _lock = lock!(lock);
                debug!("Received workspace destroy: {data:?}");
                send!(tx, WorkspaceUpdate::Remove(data.workspace_id as i64));
            });
        }

        {
            let tx = tx.clone();
            let lock = lock.clone();

            event_listener.add_urgent_state_handler(move |address| {
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
                        send!(
                            tx,
                            WorkspaceUpdate::Urgent {
                                id: c.workspace.id as i64,
                                urgent: true,
                            }
                        );
                    },
                );
            });
        }
    }

    #[cfg(feature = "keyboard+hyprland")]
    fn listen_keyboard_events(
        keyboard_layout_tx: Sender<KeyboardLayoutUpdate>,
        event_listener: &mut EventListener,
        lock: std::sync::Arc<std::sync::Mutex<()>>,
    ) {
        let tx = keyboard_layout_tx.clone();
        let lock = lock.clone();

        event_listener.add_keyboard_layout_change_handler(move |layout_event| {
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

            send!(tx, KeyboardLayoutUpdate(layout));
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
        send!(
            tx,
            WorkspaceUpdate::Focus {
                old: prev_workspace.take(),
                new: workspace.clone(),
            }
        );

        send!(
            tx,
            WorkspaceUpdate::Urgent {
                id: workspace.id,
                urgent: false,
            }
        );

        prev_workspace.replace(workspace);
    }

    /// Gets a workspace by name from the server, given the active workspace if known.
    #[cfg(feature = "workspaces+hyprland")]
    fn get_workspace(name: &str, active: Option<&Workspace>) -> Option<Workspace> {
        Workspaces::get()
            .expect("Failed to get workspaces")
            .into_iter()
            .find_map(|w| {
                if w.name == name {
                    let vis = Visibility::from((&w, active.map(|w| w.name.as_ref()), &|w| {
                        create_is_visible()(w)
                    }));

                    Some(Workspace::from((vis, w)))
                } else {
                    None
                }
            })
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

        let workspaces = Workspaces::get()
            .expect("Failed to get workspaces")
            .into_iter()
            .map(|w| {
                let vis = Visibility::from((&w, active_id.as_deref(), &is_visible));

                Workspace::from((vis, w))
            })
            .collect();

        send!(self.workspace.tx, WorkspaceUpdate::Init(workspaces));

        rx
    }
}

#[cfg(feature = "keyboard+hyprland")]
use super::{KeyboardLayoutClient, KeyboardLayoutUpdate};

#[cfg(feature = "keyboard+hyprland")]
impl KeyboardLayoutClient for Client {
    fn set_next_active(&self) {
        let device = Devices::get()
            .expect("Failed to get devices")
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

        let layout = Devices::get()
            .expect("Failed to get devices")
            .keyboards
            .iter()
            .find(|k| k.main)
            .map(|k| k.active_keymap.clone());

        if let Some(layout) = layout {
            send!(self.keyboard_layout.tx, KeyboardLayoutUpdate(layout));
        } else {
            error!("Failed to get current keyboard layout hyprland");
        }

        rx
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
