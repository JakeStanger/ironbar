use crate::{await_sync, clients::compositor::Visibility, send, spawn};
use color_eyre::eyre::Result;
use std::str::FromStr;
use tokio::time::Duration;
use tracing::{error, warn};

use tokio::sync::broadcast;

use super::{Workspace as IronWorkspace, WorkspaceClient, WorkspaceUpdate};
mod connection;

use connection::{Action, Connection, Event, Request, WorkspaceReferenceArg};

#[derive(Debug)]
pub struct Client;

impl Client {
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceClient for Client {
    fn focus(&self, name: String) -> Result<()> {
        await_sync(async {
            if let Ok(mut conn) = Connection::connect().await {
                let command = Request::Action(Action::FocusWorkspace {
                    reference: WorkspaceReferenceArg::from_str(name.as_str()).unwrap(),
                });
                let result = conn.send(command).await;

                if let Ok((reply, _)) = result {
                    if reply.is_err() {
                        error!("Error in command sent to Niri");
                    }
                } else {
                    error!("Unable to send request");
                }
            } else {
                error!("Unable to create connection to Niri socket");
            }
        });
        Ok(())
    }

    fn subscribe_workspace_change(
        &self,
    ) -> tokio::sync::broadcast::Receiver<super::WorkspaceUpdate> {
        let (tx, rx) = broadcast::channel(32);

        spawn(async move {
            if let Ok(mut conn) = Connection::connect().await {
                let reply = conn.send(Request::EventStream).await;
                if let Ok((_, mut event_listener)) = reply {
                    let mut workspace_state: Vec<IronWorkspace> = Vec::new();
                    let mut first_event = true;
                    loop {
                        if let Ok(event_niri) = event_listener() {
                            let events = match event_niri {
                                Event::WorkspacesChanged { workspaces } => {
                                    // Niri only has a WorkspacesChanged Event and Ironbar has 4 events which have to be handled: Add, Remove, Rename and Move.
                                    // This is handled by keeping a previous state of workspaces and comparing with the new state for changes.
                                    let mut new_workspaces: Vec<IronWorkspace> = workspaces
                                        .into_iter()
                                        .map(|w| IronWorkspace::from(&w))
                                        .collect();
                                    let mut updates: Vec<WorkspaceUpdate> = vec![];
                                    if first_event {
                                        updates.push(WorkspaceUpdate::Init(new_workspaces.clone()));
                                        first_event = false;
                                    } else {
                                        // First the new workspace state is sorted based on id.
                                        new_workspaces.sort_by_key(|w| w.id);
                                        let mut old_index = 0;
                                        let mut new_index = 0;
                                        // Then a linear scan on the states(old and new)  together.
                                        while old_index < workspace_state.len()
                                            && new_index < new_workspaces.len()
                                        {
                                            let old_workspace = &workspace_state[old_index];
                                            let new_workspace = &new_workspaces[new_index];
                                            match old_workspace.id.cmp(&new_workspace.id) {
                                                std::cmp::Ordering::Greater => {
                                                    //  If there is a new id, then a WorkspaceUpdate::Add event is sent.
                                                    updates.push(WorkspaceUpdate::Add(
                                                        new_workspace.clone(),
                                                    ));
                                                    new_index += 1;
                                                }
                                                //  If an id is missing, then a WorkspaceUpdate::Remove event is sent.
                                                std::cmp::Ordering::Less => {
                                                    updates.push(WorkspaceUpdate::Remove(
                                                        old_workspace.id,
                                                    ));
                                                    old_index += 1;
                                                }
                                                std::cmp::Ordering::Equal => {
                                                    // For workspaces with the same id, if the name of the workspace is different, WorkspaceUpdate::Rename is sent, if the name of the monitor is different then WorkspaceUpdate::Move is sent.
                                                    if old_workspace.name != new_workspace.name {
                                                        updates.push(WorkspaceUpdate::Rename {
                                                            id: new_workspace.id,
                                                            name: new_workspace.name.clone(),
                                                        });
                                                    }
                                                    if old_workspace.monitor
                                                        != new_workspace.monitor
                                                    {
                                                        updates.push(WorkspaceUpdate::Move(
                                                            new_workspace.clone(),
                                                        ));
                                                    }
                                                    old_index += 1;
                                                    new_index += 1;
                                                }
                                            }
                                        }
                                        // Handle remaining workspaces
                                        while old_index < workspace_state.len() {
                                            updates.push(WorkspaceUpdate::Remove(
                                                workspace_state[old_index].id,
                                            ));
                                            old_index += 1;
                                        }
                                        while new_index < new_workspaces.len() {
                                            updates.push(WorkspaceUpdate::Add(
                                                new_workspaces[new_index].clone(),
                                            ));
                                            new_index += 1;
                                        }
                                    }
                                    // At the end, over write the old workspace state with the new one. Because of this, on the next event, the old workspace state is already sorted.
                                    workspace_state = new_workspaces;
                                    updates
                                }
                                Event::WorkspaceActivated { id, focused } => {
                                    // workspace with id is activated, if focus is true then it is also focused
                                    // if focuesd is true then focus has changed => find old focused workspace. set it to inactive and set current
                                    match workspace_state.iter().position(|w| w.id == id as i64) {
                                        Some(new_index) => {
                                            if focused {
                                                match workspace_state
                                                    .iter()
                                                    .position(|w| w.visibility.is_focused())
                                                {
                                                    Some(old_index) => {
                                                        workspace_state[new_index].visibility =
                                                            Visibility::focused();
                                                        if workspace_state[old_index].monitor
                                                            == workspace_state[new_index].monitor
                                                        {
                                                            workspace_state[old_index].visibility =
                                                                Visibility::Hidden;
                                                        } else {
                                                            workspace_state[old_index].visibility =
                                                                Visibility::visible();
                                                        }
                                                        vec![WorkspaceUpdate::Focus {
                                                            old: Some(
                                                                workspace_state[old_index].clone(),
                                                            ),
                                                            new: workspace_state[new_index].clone(),
                                                        }]
                                                    }
                                                    None => {
                                                        workspace_state[new_index].visibility =
                                                            Visibility::focused();
                                                        vec![WorkspaceUpdate::Focus {
                                                            old: None,
                                                            new: workspace_state[new_index].clone(),
                                                        }]
                                                    }
                                                }
                                            } else {
                                                // if focused is false means active workspace on a particular monitor has changed => change all workspaces on monitor to inactive and change current workspace as active
                                                workspace_state[new_index].visibility =
                                                    Visibility::visible();
                                                match workspace_state.iter().position(|w| {
                                                    (w.visibility.is_focused()
                                                        || w.visibility.is_visible())
                                                        && w.monitor
                                                            == workspace_state[new_index].monitor
                                                }) {
                                                    Some(old_index) => {
                                                        workspace_state[old_index].visibility =
                                                            Visibility::Hidden;
                                                        vec![]
                                                    }
                                                    None => {
                                                        vec![]
                                                    }
                                                }
                                            }
                                        }
                                        None => {
                                            warn!(
                                    "No workspace with id for new focus/visible workspace found"
                                );
                                            vec![]
                                        }
                                    }
                                }
                                Event::Other => {
                                    vec![]
                                }
                            };
                            for event in events {
                                send!(tx, event);
                            }
                            tokio::time::sleep(Duration::from_millis(30)).await;
                        }
                    }
                }
            } else {
                error!("Unable to create connection to Niri socket");
            }
        });
        rx
    }
}
