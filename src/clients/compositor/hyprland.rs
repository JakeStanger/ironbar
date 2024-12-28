use super::{Visibility, Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::channels::SyncSenderExt;
use crate::{arc_mut, lock, spawn_blocking};
use color_eyre::Result;
use hyprland::data::{Workspace as HWorkspace, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::event_listener::EventListener;
use hyprland::prelude::*;
use hyprland::shared::{HyprDataVec, WorkspaceType};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct Client {
    workspace_tx: Sender<WorkspaceUpdate>,
    _workspace_rx: Receiver<WorkspaceUpdate>,
}

impl Client {
    pub(crate) fn new() -> Self {
        let (workspace_tx, workspace_rx) = channel(16);

        let instance = Self {
            workspace_tx,
            _workspace_rx: workspace_rx,
        };

        instance.listen_workspace_events();
        instance
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

                    let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                    if let Some(workspace) = workspace {
                        tx.send_expect(WorkspaceUpdate::Add(workspace));
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

                    if let Some((false, workspace)) =
                        workspace.map(|w| (w.visibility.is_focused(), w))
                    {
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
                    let workspace = Self::get_workspace(&workspace_name, prev_workspace.as_ref());

                    if let Some(workspace) = workspace {
                        tx.send_expect(WorkspaceUpdate::Move(workspace.clone()));

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

                    tx.send_expect(WorkspaceUpdate::Rename {
                        id: data.workspace_id as i64,
                        name: data.workspace_name,
                    });
                });
            }

            {
                let tx = tx.clone();
                let lock = lock.clone();

                event_listener.add_workspace_destroy_handler(move |data| {
                    let _lock = lock!(lock);
                    debug!("Received workspace destroy: {data:?}");
                    tx.send_expect(WorkspaceUpdate::Remove(data.workspace_id as i64));
                });
            }

            {
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
                            tx.send_expect(WorkspaceUpdate::Urgent {
                                id: c.workspace.id as i64,
                                urgent: true,
                            });
                        },
                    );
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

impl WorkspaceClient for Client {
    fn focus(&self, id: String) -> Result<()> {
        let identifier = id.parse::<i32>().map_or_else(
            |_| WorkspaceIdentifierWithSpecial::Name(&id),
            WorkspaceIdentifierWithSpecial::Id,
        );

        Dispatch::call(DispatchType::Workspace(identifier))?;
        Ok(())
    }

    fn subscribe_workspace_change(&self) -> Receiver<WorkspaceUpdate> {
        let rx = self.workspace_tx.subscribe();

        {
            let tx = self.workspace_tx.clone();

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

            tx.send_expect(WorkspaceUpdate::Init(workspaces));
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
