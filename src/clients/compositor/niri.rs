use crate::{
    await_sync,
    clients::{
        compositor::Visibility,
        niri::{Action, Connection, Event, Request, WorkspaceReferenceArg},
    },
    send, spawn,
};
use color_eyre::eyre::Result;
use std::{str::FromStr, time::Duration};
use tokio::sync::broadcast::channel;

use super::{Workspace, WorkspaceClient, WorkspaceUpdate};

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
            let mut conn = Connection::connect().await.unwrap();

            let command = Request::Action(Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::from_str(name.as_str()).unwrap(),
            });
            conn.send(command).await.unwrap().0.unwrap();
        });
        Ok(())
    }

    fn subscribe_workspace_change(
        &self,
    ) -> tokio::sync::broadcast::Receiver<super::WorkspaceUpdate> {
        let (tx, rx) = channel(32);

        spawn(async move {
            let mut conn = Connection::connect().await.unwrap();
            let mut event_listener = conn.send(Request::EventStream).await.unwrap().1;
            let mut workspace_state: Vec<Workspace> = Vec::new();
            let mut first_event = true;
            loop {
                let event_niri = event_listener().unwrap();
                let events = match event_niri {
                    Event::WorkspacesChanged { workspaces } => {
                        let mut new_workspaces: Vec<Workspace> = workspaces
                            .into_iter()
                            .map(|w| Workspace::from(&w))
                            .collect();
                        let mut updates: Vec<WorkspaceUpdate> = vec![];
                        if first_event {
                            updates.push(WorkspaceUpdate::Init(new_workspaces.clone()));
                            first_event = false;
                        } else {
                            new_workspaces.sort_by_key(|w| w.id);
                            workspace_state.sort_by_key(|w| w.id);
                            let mut old_index = 0;
                            let mut new_index = 0;
                            while old_index < workspace_state.len()
                                && new_index < new_workspaces.len()
                            {
                                let old_workspace = &workspace_state[old_index];
                                let new_workspace = &new_workspaces[new_index];
                                match old_workspace.id.cmp(&new_workspace.id) {
                                    std::cmp::Ordering::Greater => {
                                        updates.push(WorkspaceUpdate::Add(new_workspace.clone()));
                                        new_index += 1;
                                    }
                                    std::cmp::Ordering::Less => {
                                        updates.push(WorkspaceUpdate::Remove(old_workspace.id));
                                        old_index += 1;
                                    }
                                    std::cmp::Ordering::Equal => {
                                        let mut rename = false;
                                        let mut mv = false;
                                        if old_workspace.name != new_workspace.name {
                                            updates.push(WorkspaceUpdate::Rename {
                                                id: new_workspace.id,
                                                name: new_workspace.name.clone(),
                                            });
                                            rename = true;
                                        }
                                        if old_workspace.monitor != new_workspace.monitor {
                                            updates
                                                .push(WorkspaceUpdate::Move(new_workspace.clone()));
                                            mv = true;
                                        }
                                        if rename || mv {
                                            workspace_state[old_index] = new_workspace.clone();
                                        }
                                        old_index += 1;
                                        new_index += 1;
                                    }
                                }
                            }
                            while old_index < workspace_state.len() {
                                updates
                                    .push(WorkspaceUpdate::Remove(workspace_state[old_index].id));
                                old_index += 1;
                            }
                            while new_index < new_workspaces.len() {
                                updates
                                    .push(WorkspaceUpdate::Add(new_workspaces[new_index].clone()));
                                new_index += 1;
                            }
                        }
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
                                                old: Some(workspace_state[old_index].clone()),
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
                                    workspace_state[new_index].visibility = Visibility::visible();
                                    match workspace_state.iter().position(|w| {
                                        (w.visibility.is_focused() || w.visibility.is_visible())
                                            && w.monitor == workspace_state[new_index].monitor
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
                                tracing::warn!(
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
                std::thread::sleep(Duration::from_millis(30));
            }
        });
        rx
    }
}
