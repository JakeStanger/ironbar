use crate::{clients::compositor::Visibility, send, spawn};
use color_eyre::Report;
use tracing::{error, warn};

use tokio::sync::broadcast;

use super::{Workspace as IronWorkspace, WorkspaceClient, WorkspaceUpdate};
mod connection;

use connection::{Action, Connection, Event, Request, WorkspaceReferenceArg};

#[derive(Debug)]
pub struct Client {
    tx: broadcast::Sender<WorkspaceUpdate>,
    _rx: broadcast::Receiver<WorkspaceUpdate>,
}

impl Client {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel(32);
        let tx2 = tx.clone();

        spawn(async move {
            let mut conn = Connection::connect().await?;
            let (_, mut event_listener) = conn.send(Request::EventStream).await?;

            let mut workspace_state: Vec<IronWorkspace> = Vec::new();
            let mut first_event = true;

            loop {
                let events = match event_listener() {
                    Ok(Event::WorkspacesChanged { workspaces }) => {
                        // Niri only has a WorkspacesChanged Event and Ironbar has 4 events which have to be handled: Add, Remove, Rename and Move.
                        // This is handled by keeping a previous state of workspaces and comparing with the new state for changes.
                        let new_workspaces: Vec<IronWorkspace> = workspaces
                            .into_iter()
                            .map(|w| IronWorkspace::from(&w))
                            .collect();

                        let mut updates: Vec<WorkspaceUpdate> = vec![];

                        if first_event {
                            let mut new_workspaces = new_workspaces.clone();
                            new_workspaces.sort_by_key(|w| w.id);
                            updates.push(WorkspaceUpdate::Init(new_workspaces));
                            first_event = false;
                        } else {
                            // first pass - add/update
                            for workspace in &new_workspaces {
                                let old_workspace =
                                    workspace_state.iter().find(|w| w.id == workspace.id);

                                match old_workspace {
                                    None => updates.push(WorkspaceUpdate::Add(workspace.clone())),
                                    Some(old_workspace) => {
                                        if workspace.name != old_workspace.name {
                                            updates.push(WorkspaceUpdate::Rename {
                                                id: workspace.id,
                                                name: workspace.name.clone(),
                                            });
                                        }

                                        if workspace.monitor != old_workspace.monitor {
                                            updates.push(WorkspaceUpdate::Move(workspace.clone()));
                                        }
                                    }
                                }
                            }

                            // second pass - delete
                            for workspace in &workspace_state {
                                let exists = new_workspaces.iter().any(|w| w.id == workspace.id);

                                if !exists {
                                    updates.push(WorkspaceUpdate::Remove(workspace.id));
                                }
                            }
                        }

                        workspace_state = new_workspaces;
                        updates
                    }

                    Ok(Event::WorkspaceActivated { id, focused }) => {
                        // workspace with id is activated, if focus is true then it is also focused
                        // if focused is true then focus has changed => find old focused workspace. set it to inactive and set current
                        //
                        // we use indexes here as both new/old need to be mutable

                        if let Some(new_index) =
                            workspace_state.iter().position(|w| w.id == id as i64)
                        {
                            if focused {
                                if let Some(old_index) = workspace_state
                                    .iter()
                                    .position(|w| w.visibility.is_focused())
                                {
                                    workspace_state[new_index].visibility = Visibility::focused();

                                    if workspace_state[old_index].monitor
                                        == workspace_state[new_index].monitor
                                    {
                                        workspace_state[old_index].visibility = Visibility::Hidden;
                                    } else {
                                        workspace_state[old_index].visibility =
                                            Visibility::visible();
                                    }

                                    vec![WorkspaceUpdate::Focus {
                                        old: Some(workspace_state[old_index].clone()),
                                        new: workspace_state[new_index].clone(),
                                    }]
                                } else {
                                    workspace_state[new_index].visibility = Visibility::focused();

                                    vec![WorkspaceUpdate::Focus {
                                        old: None,
                                        new: workspace_state[new_index].clone(),
                                    }]
                                }
                            } else {
                                // if focused is false means active workspace on a particular monitor has changed =>
                                // change all workspaces on monitor to inactive and change current workspace as active
                                workspace_state[new_index].visibility = Visibility::visible();

                                if let Some(old_index) = workspace_state.iter().position(|w| {
                                    (w.visibility.is_focused() || w.visibility.is_visible())
                                        && w.monitor == workspace_state[new_index].monitor
                                }) {
                                    workspace_state[old_index].visibility = Visibility::Hidden;

                                    vec![]
                                } else {
                                    vec![]
                                }
                            }
                        } else {
                            warn!("No workspace with id for new focus/visible workspace found");
                            vec![]
                        }
                    }
                    Ok(Event::Other) => {
                        vec![]
                    }
                    Err(err) => {
                        error!("{err:?}");
                        break;
                    }
                };

                for event in events {
                    send!(tx, event);
                }
            }

            Ok::<(), Report>(())
        });

        Self { tx: tx2, _rx: rx }
    }
}

impl WorkspaceClient for Client {
    fn focus(&self, id: i64) {
        // this does annoyingly require spawning a separate connection for every focus call
        // the alternative is sticking the conn behind a mutex which could perform worse
        spawn(async move {
            let mut conn = Connection::connect().await?;

            let command = Request::Action(Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Id(id as u64),
            });

            if let Err(err) = conn.send(command).await {
                error!("failed to send command: {err:?}");
            }

            Ok::<(), Report>(())
        });
    }

    fn subscribe(&self) -> broadcast::Receiver<WorkspaceUpdate> {
        self.tx.subscribe()
    }
}
