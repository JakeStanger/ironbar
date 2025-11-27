use super::{Workspace as IronWorkspace, WorkspaceClient, WorkspaceUpdate};
use crate::channels::SyncSenderExt;
use crate::clients::compositor::Visibility;
use crate::{arc_rw, read_lock, spawn, write_lock};
use connection::{Action, Connection, Event, Request, WorkspaceReferenceArg};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

mod connection;

#[derive(Debug)]
pub struct Client {
    tx: broadcast::Sender<WorkspaceUpdate>,
    _rx: broadcast::Receiver<WorkspaceUpdate>,

    workspaces: Arc<RwLock<Vec<IronWorkspace>>>,
}

impl Client {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel(32);
        let tx2 = tx.clone();

        let workspace_state = arc_rw!(vec![]);
        let workspace_state2 = workspace_state.clone();

        spawn(async move {
            let mut conn = Connection::connect().await?;
            let (_, mut event_listener) = conn.send(Request::EventStream).await?;

            let mut first_event = true;

            loop {
                let events = match event_listener() {
                    Ok(Event::WorkspacesChanged { workspaces }) => {
                        debug!("WorkspacesChanged: {:?}", workspaces);

                        // Niri only has a WorkspacesChanged Event and Ironbar has 4 events which have to be handled: Add, Remove, Rename and Move.
                        // This is handled by keeping a previous state of workspaces and comparing with the new state for changes.
                        let new_workspaces: Vec<IronWorkspace> = workspaces
                            .into_iter()
                            .map(|w| IronWorkspace::from(&w))
                            .collect();

                        let mut updates: Vec<WorkspaceUpdate> = vec![];

                        if first_event {
                            // Niri's WorkspacesChanged event does not initially sort workspaces by ID when first output,
                            // which makes sort = added meaningless. Therefore, new_workspaces are sorted by ID here to ensure a consistent addition order.
                            let mut new_workspaces = new_workspaces.clone();
                            new_workspaces.sort_by_key(|w| w.id);
                            updates.push(WorkspaceUpdate::Init(new_workspaces));
                            first_event = false;
                        } else {
                            // first pass - add/update
                            for workspace in &new_workspaces {
                                let workspace_state = read_lock!(workspace_state);
                                let old_workspace = workspace_state
                                    .iter()
                                    .find(|&w: &&IronWorkspace| w.id == workspace.id);

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
                            for workspace in read_lock!(workspace_state).iter() {
                                let exists = new_workspaces.iter().any(|w| w.id == workspace.id);

                                if !exists {
                                    updates.push(WorkspaceUpdate::Remove(workspace.id));
                                }
                            }
                        }

                        *write_lock!(workspace_state) = new_workspaces;
                        updates
                    }

                    Ok(Event::WorkspaceActivated { id, focused }) => {
                        debug!("WorkspaceActivated: id: {}, focused: {}", id, focused);

                        // workspace with id is activated, if focus is true then it is also focused
                        // if focused is true then focus has changed => find old focused workspace. set it to inactive and set current
                        //
                        // we use indexes here as both new/old need to be mutable

                        let new_index = read_lock!(workspace_state)
                            .iter()
                            .position(|w| w.id == id as i64);

                        if let Some(new_index) = new_index {
                            if focused {
                                let old_index = read_lock!(workspace_state)
                                    .iter()
                                    .position(|w| w.visibility.is_focused());

                                if let Some(old_index) = old_index {
                                    write_lock!(workspace_state)[new_index].visibility =
                                        Visibility::focused();

                                    if read_lock!(workspace_state)[old_index].monitor
                                        == read_lock!(workspace_state)[new_index].monitor
                                    {
                                        write_lock!(workspace_state)[old_index].visibility =
                                            Visibility::Hidden;
                                    } else {
                                        write_lock!(workspace_state)[old_index].visibility =
                                            Visibility::visible();
                                    }

                                    vec![WorkspaceUpdate::Focus {
                                        old: Some(read_lock!(workspace_state)[old_index].clone()),
                                        new: read_lock!(workspace_state)[new_index].clone(),
                                    }]
                                } else {
                                    write_lock!(workspace_state)[new_index].visibility =
                                        Visibility::focused();

                                    vec![WorkspaceUpdate::Focus {
                                        old: None,
                                        new: read_lock!(workspace_state)[new_index].clone(),
                                    }]
                                }
                            } else {
                                // if focused is false means active workspace on a particular monitor has changed =>
                                // change all workspaces on monitor to inactive and change current workspace as active
                                write_lock!(workspace_state)[new_index].visibility =
                                    Visibility::visible();

                                let old_index = read_lock!(workspace_state).iter().position(|w| {
                                    (w.visibility.is_focused() || w.visibility.is_visible())
                                        && w.monitor
                                            == read_lock!(workspace_state)[new_index].monitor
                                });

                                if let Some(old_index) = old_index {
                                    write_lock!(workspace_state)[old_index].visibility =
                                        Visibility::Hidden;

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
                    Ok(Event::WorkspaceUrgencyChanged { id, urgent }) => {
                        vec![WorkspaceUpdate::Urgent {
                            id: id as i64,
                            urgent,
                        }]
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
                    tx.send_expect(event);
                }
            }

            Ok::<(), std::io::Error>(())
        });

        Self {
            tx: tx2,
            _rx: rx,
            workspaces: workspace_state2,
        }
    }
}

impl WorkspaceClient for Client {
    fn focus(&self, id: i64) {
        debug!("focusing workspace with id: {}", id);

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

            Ok::<(), std::io::Error>(())
        });
    }

    fn subscribe(&self) -> broadcast::Receiver<WorkspaceUpdate> {
        let rx = self.tx.subscribe();

        let workspaces = read_lock!(self.workspaces);
        if !workspaces.is_empty() {
            self.tx
                .send_expect(WorkspaceUpdate::Init(workspaces.clone()));
        }

        rx
    }
}
