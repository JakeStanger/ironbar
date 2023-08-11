use super::{Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::{arc_mut, lock, send};
use color_eyre::Result;
use hyprland::data::{Workspace as HWorkspace, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::event_listener::EventListener;
use hyprland::prelude::*;
use hyprland::shared::WorkspaceType;
use lazy_static::lazy_static;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::task::spawn_blocking;
use tracing::{debug, error, info};

pub struct EventClient {
    workspace_tx: Sender<WorkspaceUpdate>,
    _workspace_rx: Receiver<WorkspaceUpdate>,
}

impl EventClient {
    fn new() -> Self {
        let (workspace_tx, workspace_rx) = channel(16);

        Self {
            workspace_tx,
            _workspace_rx: workspace_rx,
        }
    }

    fn listen_workspace_events(&self) {
        info!("Starting Hyprland event listener");

        let tx = self.workspace_tx.clone();

        spawn_blocking(move || {
            let mut event_listener = EventListener::new();

            // we need a lock to ensure events don't run at the same time
            let lock = arc_mut!(());

            // cache the active workspace since Hyprland doesn't give us the prev active
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
                    let focused = prev_workspace
                        .as_ref()
                        .map_or(false, |w| w.name == workspace_name);

                    let workspace = Self::get_workspace(&workspace_name, focused);

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
                    let focused = prev_workspace
                        .as_ref()
                        .map_or(false, |w| w.name == workspace_name);
                    let workspace = Self::get_workspace(&workspace_name, focused);

                    workspace.map_or_else(
                        || {
                            error!("Unable to locate workspace");
                        },
                        |workspace| {
                            // there may be another type of update so dispatch that regardless of focus change
                            send!(tx, WorkspaceUpdate::Update(workspace.clone()));
                            if !focused {
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
                    let focused = prev_workspace
                        .as_ref()
                        .map_or(false, |w| w.name == workspace_name);
                    let workspace = Self::get_workspace(&workspace_name, focused);

                    if let (Some(workspace), false) = (workspace, focused) {
                        Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                    } else {
                        error!("Unable to locate workspace");
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
                    let focused = prev_workspace
                        .as_ref()
                        .map_or(false, |w| w.name == workspace_name);
                    let workspace = Self::get_workspace(&workspace_name, focused);

                    if let Some(workspace) = workspace {
                        send!(tx, WorkspaceUpdate::Move(workspace.clone()));

                        if !focused {
                            Self::send_focus_change(&mut prev_workspace, workspace, &tx);
                        }
                    }
                });
            }

            {
                event_listener.add_workspace_destroy_handler(move |workspace_type| {
                    let _lock = lock!(lock);
                    debug!("Received workspace destroy: {workspace_type:?}");

                    let name = get_workspace_name(workspace_type);
                    send!(tx, WorkspaceUpdate::Remove(name));
                });
            }

            event_listener
                .start_listener()
                .expect("Failed to start listener");
        });
    }

    /// Sends a `WorkspaceUpdate::Focus` event
    /// and updates the active workspace cache.
    fn send_focus_change(
        prev_workspace: &mut Option<Workspace>,
        workspace: Workspace,
        tx: &Sender<WorkspaceUpdate>,
    ) {
        let old = prev_workspace
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_default();

        send!(
            tx,
            WorkspaceUpdate::Focus {
                old,
                new: workspace.name.clone(),
            }
        );

        prev_workspace.replace(workspace);
    }

    /// Gets a workspace by name from the server.
    ///
    /// Use `focused` to manually mark the workspace as focused,
    /// as this is not automatically checked.
    fn get_workspace(name: &str, focused: bool) -> Option<Workspace> {
        Workspaces::get()
            .expect("Failed to get workspaces")
            .find_map(|w| {
                if w.name == name {
                    Some(Workspace::from((focused, w)))
                } else {
                    None
                }
            })
    }

    /// Gets the active workspace from the server.
    fn get_active_workspace() -> Result<Workspace> {
        let w = HWorkspace::get_active().map(|w| Workspace::from((true, w)))?;
        Ok(w)
    }
}

impl WorkspaceClient for EventClient {
    fn focus(&self, id: String) -> Result<()> {
        let identifier = match id.parse::<i32>() {
            Ok(inum) => WorkspaceIdentifierWithSpecial::Id(inum),
            Err(_) => WorkspaceIdentifierWithSpecial::Name(&id),
        };

        Dispatch::call(DispatchType::Workspace(identifier))?;
        Ok(())
    }

    fn subscribe_workspace_change(&self) -> Receiver<WorkspaceUpdate> {
        let rx = self.workspace_tx.subscribe();

        {
            let tx = self.workspace_tx.clone();

            let active_name = HWorkspace::get_active()
                .map(|active| active.name)
                .unwrap_or_default();

            let workspaces = Workspaces::get()
                .expect("Failed to get workspaces")
                .map(|w| Workspace::from((w.name == active_name, w)))
                .collect();

            send!(tx, WorkspaceUpdate::Init(workspaces));
        }

        rx
    }
}

lazy_static! {
    static ref CLIENT: EventClient = {
        let client = EventClient::new();
        client.listen_workspace_events();
        client
    };
}

pub fn get_client() -> &'static EventClient {
    &CLIENT
}

fn get_workspace_name(name: WorkspaceType) -> String {
    match name {
        WorkspaceType::Regular(name) => name,
        WorkspaceType::Special(name) => name.unwrap_or_default(),
    }
}

impl From<(bool, hyprland::data::Workspace)> for Workspace {
    fn from((focused, workspace): (bool, hyprland::data::Workspace)) -> Self {
        Self {
            id: workspace.id.to_string(),
            name: workspace.name,
            monitor: workspace.monitor,
            focused,
        }
    }
}
