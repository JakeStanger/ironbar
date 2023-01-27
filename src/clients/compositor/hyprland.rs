use super::{Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::error::{ERR_CHANNEL_SEND, ERR_MUTEX_LOCK};
use hyprland::data::{Workspace as HWorkspace, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::event_listener::EventListenerMutable as EventListener;
use hyprland::prelude::*;
use hyprland::shared::WorkspaceType;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::task::spawn_blocking;
use tracing::{error, info};

pub struct EventClient {
    workspaces: Arc<Mutex<Vec<Workspace>>>,
    workspace_tx: Sender<WorkspaceUpdate>,
    _workspace_rx: Receiver<WorkspaceUpdate>,
}

impl EventClient {
    fn new() -> Self {
        let (workspace_tx, workspace_rx) = channel(16);

        let workspaces = Arc::new(Mutex::new(vec![]));
        // load initial list
        Self::refresh_workspaces(&workspaces);

        Self {
            workspaces,
            workspace_tx,
            _workspace_rx: workspace_rx,
        }
    }

    fn listen_workspace_events(&self) {
        info!("Starting Hyprland event listener");

        let workspaces = self.workspaces.clone();
        let tx = self.workspace_tx.clone();

        spawn_blocking(move || {
            let mut event_listener = EventListener::new();

            {
                let workspaces = workspaces.clone();
                let tx = tx.clone();

                event_listener.add_workspace_added_handler(move |workspace_type, _state| {
                    Self::refresh_workspaces(&workspaces);

                    let workspace = Self::get_workspace(&workspaces, workspace_type);
                    workspace.map_or_else(
                        || error!("Unable to locate workspace"),
                        |workspace| {
                            tx.send(WorkspaceUpdate::Add(workspace))
                                .expect(ERR_CHANNEL_SEND);
                        },
                    );
                });
            }

            {
                let workspaces = workspaces.clone();
                let tx = tx.clone();

                event_listener.add_workspace_change_handler(move |workspace_type, _state| {
                    let prev_workspace = Self::get_focused_workspace(&workspaces);

                    Self::refresh_workspaces(&workspaces);

                    let workspace = Self::get_workspace(&workspaces, workspace_type);

                    if let (Some(prev_workspace), Some(workspace)) = (prev_workspace, workspace) {
                        if prev_workspace.id != workspace.id {
                            tx.send(WorkspaceUpdate::Focus {
                                old: prev_workspace,
                                new: workspace.clone(),
                            })
                            .expect(ERR_CHANNEL_SEND);
                        }

                        // there may be another type of update so dispatch that regardless of focus change
                        tx.send(WorkspaceUpdate::Update(workspace))
                            .expect(ERR_CHANNEL_SEND);
                    } else {
                        error!("Unable to locate workspace");
                    }
                });
            }

            {
                let workspaces = workspaces.clone();
                let tx = tx.clone();

                event_listener.add_workspace_destroy_handler(move |workspace_type, _state| {
                    let workspace = Self::get_workspace(&workspaces, workspace_type);
                    workspace.map_or_else(
                        || error!("Unable to locate workspace"),
                        |workspace| {
                            tx.send(WorkspaceUpdate::Remove(workspace))
                                .expect(ERR_CHANNEL_SEND);
                        },
                    );

                    Self::refresh_workspaces(&workspaces);
                });
            }

            {
                let workspaces = workspaces.clone();
                let tx = tx.clone();

                event_listener.add_workspace_moved_handler(move |event_data, _state| {
                    let workspace_type = event_data.1;

                    Self::refresh_workspaces(&workspaces);

                    let workspace = Self::get_workspace(&workspaces, workspace_type);
                    workspace.map_or_else(
                        || error!("Unable to locate workspace"),
                        |workspace| {
                            tx.send(WorkspaceUpdate::Move(workspace))
                                .expect(ERR_CHANNEL_SEND);
                        },
                    );
                });
            }

            {
                let workspaces = workspaces.clone();

                event_listener.add_active_monitor_change_handler(move |event_data, _state| {
                    let workspace_type = event_data.1;

                    let prev_workspace = Self::get_focused_workspace(&workspaces);

                    Self::refresh_workspaces(&workspaces);

                    let workspace = Self::get_workspace(&workspaces, workspace_type);

                    if let (Some(prev_workspace), Some(workspace)) = (prev_workspace, workspace) {
                        if prev_workspace.id != workspace.id {
                            tx.send(WorkspaceUpdate::Focus {
                                old: prev_workspace,
                                new: workspace,
                            })
                            .expect(ERR_CHANNEL_SEND);
                        }
                    } else {
                        error!("Unable to locate workspace");
                    }
                });
            }

            event_listener
                .start_listener()
                .expect("Failed to start listener");
        });
    }

    fn refresh_workspaces(workspaces: &Mutex<Vec<Workspace>>) {
        let mut workspaces = workspaces.lock().expect(ERR_MUTEX_LOCK);

        let active = HWorkspace::get_active().expect("Failed to get active workspace");
        let new_workspaces = Workspaces::get()
            .expect("Failed to get workspaces")
            .collect()
            .into_iter()
            .map(|workspace| Workspace::from((workspace.id == active.id, workspace)));

        workspaces.clear();
        workspaces.extend(new_workspaces);
    }

    fn get_workspace(workspaces: &Mutex<Vec<Workspace>>, id: WorkspaceType) -> Option<Workspace> {
        let id_string = id_to_string(id);

        let workspaces = workspaces.lock().expect(ERR_MUTEX_LOCK);
        workspaces
            .iter()
            .find(|workspace| workspace.id == id_string)
            .cloned()
    }

    fn get_focused_workspace(workspaces: &Mutex<Vec<Workspace>>) -> Option<Workspace> {
        let workspaces = workspaces.lock().expect(ERR_MUTEX_LOCK);
        workspaces
            .iter()
            .find(|workspace| workspace.focused)
            .cloned()
    }
}

impl WorkspaceClient for EventClient {
    fn focus(&self, id: String) -> color_eyre::Result<()> {
        Dispatch::call(DispatchType::Workspace(
            WorkspaceIdentifierWithSpecial::Name(&id),
        ))?;
        Ok(())
    }

    fn subscribe_workspace_change(&self) -> Receiver<WorkspaceUpdate> {
        let rx = self.workspace_tx.subscribe();

        {
            let tx = self.workspace_tx.clone();

            let workspaces = self.workspaces.clone();
            Self::refresh_workspaces(&workspaces);

            let workspaces = workspaces.lock().expect(ERR_MUTEX_LOCK);

            tx.send(WorkspaceUpdate::Init(workspaces.clone()))
                .expect(ERR_CHANNEL_SEND);
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

fn id_to_string(id: WorkspaceType) -> String {
    match id {
        WorkspaceType::Unnamed(id) => id.to_string(),
        WorkspaceType::Named(name) => name,
        WorkspaceType::Special(name) => name.unwrap_or_default(),
    }
}

impl From<(bool, hyprland::data::Workspace)> for Workspace {
    fn from((focused, workspace): (bool, hyprland::data::Workspace)) -> Self {
        Self {
            id: id_to_string(workspace.id),
            name: workspace.name,
            monitor: workspace.monitor,
            focused,
        }
    }
}
