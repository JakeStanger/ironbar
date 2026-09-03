use super::Visibility;
use super::{Workspace as IronWorkspace, WorkspaceClient, WorkspaceUpdate};
use crate::channels::SyncSenderExt;
#[cfg(feature = "keyboard+niri")]
use crate::clients::compositor::{
    KeyboardLayoutClient, KeyboardLayoutUpdate,
    niri::connection::{KeyboardLayouts, LayoutSwitchTarget},
};
use crate::lock;
use crate::spawn;
use connection::{Action, Connection, Event, Request, WorkspaceReferenceArg};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

mod connection;

#[cfg(feature = "keyboard+niri")]
#[derive(Debug)]
struct KeyboardLayoutHandler {
    tx: broadcast::Sender<KeyboardLayoutUpdate>,
    _rx: broadcast::Receiver<KeyboardLayoutUpdate>,

    layouts: Mutex<KeyboardLayouts>,
}

#[cfg(feature = "keyboard+niri")]
impl KeyboardLayoutHandler {
    fn new() -> Self {
        let (tx, rx) = broadcast::channel(32);
        Self {
            tx,
            _rx: rx,
            layouts: Mutex::default(),
        }
    }

    fn handle(&self, event: Event) -> Option<Event> {
        let update = {
            let mut layouts = lock!(self.layouts);
            match event {
                Event::KeyboardLayoutsChanged { keyboard_layouts } => *layouts = keyboard_layouts,
                Event::KeyboardLayoutSwitched { idx } => layouts.current_idx = idx,
                other => return Some(other),
            }
            layouts
                .current()
                .map(|layout| KeyboardLayoutUpdate(layout.to_string()))
        };

        if let Some(update) = update {
            self.tx.send_expect(update);
        } else {
            warn!("No active keyboard layouts found");
        }

        None
    }
}

#[derive(Debug)]
struct WorkspaceHandler {
    tx: broadcast::Sender<WorkspaceUpdate>,
    _rx: broadcast::Receiver<WorkspaceUpdate>,

    workspaces: Mutex<Vec<IronWorkspace>>,
}

impl WorkspaceHandler {
    fn new() -> Self {
        let (tx, rx) = broadcast::channel(32);
        Self {
            tx,
            _rx: rx,
            workspaces: Mutex::default(),
        }
    }

    fn handle(&self, event: Event) -> Option<Event> {
        match event {
            Event::WorkspacesChanged { workspaces } => {
                debug!("WorkspacesChanged: {:?}", workspaces);

                // Niri only has a WorkspacesChanged Event and Ironbar has 4 events which have to be handled: Add, Remove, Rename and Move.
                // This is handled by keeping a previous state of workspaces and comparing with the new state for changes.
                let new_workspaces: Vec<IronWorkspace> = workspaces
                    .into_iter()
                    .map(|w| IronWorkspace::from(&w))
                    .collect();

                let updates = {
                    let mut workspaces = lock!(self.workspaces);

                    // first pass - add/update
                    let mut updates: Vec<WorkspaceUpdate> = vec![];
                    for new in &new_workspaces {
                        let old = workspaces
                            .iter()
                            .find(|&old: &&IronWorkspace| old.id == new.id);

                        match old {
                            None => updates.push(WorkspaceUpdate::Add(new.clone())),
                            Some(old) => {
                                if new.name != old.name {
                                    updates.push(WorkspaceUpdate::Rename {
                                        id: new.id,
                                        name: new.name.clone(),
                                    });
                                }

                                if new.monitor != old.monitor || new.index != old.index {
                                    updates.push(WorkspaceUpdate::Move(new.clone()));
                                }
                            }
                        }
                    }

                    // second pass - delete
                    updates.extend(
                        workspaces
                            .iter()
                            .filter(|old| !new_workspaces.iter().any(|new| new.id == old.id))
                            .map(|old| WorkspaceUpdate::Remove(old.id)),
                    );

                    *workspaces = new_workspaces;
                    updates
                };

                for update in updates {
                    self.tx.send_expect(update);
                }
            }

            Event::WorkspaceActivated { id, focused } => {
                debug!("WorkspaceActivated: id: {}, focused: {}", id, focused);

                // workspace with id is activated, if focus is true then it is also focused
                // we use indexes here as both new/old need to be mutable
                let update = {
                    let mut workspaces = lock!(self.workspaces);

                    let Some(new_index) = workspaces.iter().position(|w| w.id == id as i64) else {
                        warn!("No workspace with id for new focus/visible workspace found");
                        return None;
                    };

                    if focused {
                        // if focused is true then focus has changed => find old focused workspace,
                        // set it to inactive and change current to focused
                        let old = workspaces
                            .iter()
                            .position(|w| w.visibility.is_focused())
                            .filter(|&old_index| old_index != new_index)
                            .map(|old_index| {
                                workspaces[old_index].visibility = if workspaces[old_index].monitor
                                    == workspaces[new_index].monitor
                                {
                                    Visibility::Hidden
                                } else {
                                    Visibility::visible()
                                };
                                workspaces[old_index].clone()
                            });
                        workspaces[new_index].visibility = Visibility::focused();

                        WorkspaceUpdate::Focus {
                            old,
                            new: workspaces[new_index].clone(),
                        }
                    } else {
                        // if focused is false then active workspace on a particular monitor has changed =>
                        // change all workspaces on monitor to inactive and change current workspace to active
                        let old_index = workspaces
                            .iter()
                            .position(|old| {
                                old.visibility.is_visible()
                                    && old.monitor == workspaces[new_index].monitor
                            })
                            .filter(|&old_index| old_index != new_index);

                        if let Some(old_index) = old_index {
                            workspaces[old_index].visibility = Visibility::Hidden;
                        }

                        workspaces[new_index].visibility = Visibility::visible();
                        return None;
                    }
                };

                self.tx.send_expect(update);
            }
            Event::WorkspaceUrgencyChanged { id, urgent } => {
                self.tx.send_expect(WorkspaceUpdate::Urgent {
                    id: id as i64,
                    urgent,
                });
            }
            other => return Some(other),
        }

        None
    }
}

#[derive(Debug)]
pub struct Client {
    ws_handler: Arc<WorkspaceHandler>,

    #[cfg(feature = "keyboard+niri")]
    kb_handler: Arc<KeyboardLayoutHandler>,
}

impl Client {
    pub fn new() -> Self {
        let instance = Self {
            ws_handler: Arc::new(WorkspaceHandler::new()),
            #[cfg(feature = "keyboard+niri")]
            kb_handler: Arc::new(KeyboardLayoutHandler::new()),
        };
        let ws_handler = instance.ws_handler.clone();
        #[cfg(feature = "keyboard+niri")]
        let kb_handler = instance.kb_handler.clone();

        spawn(async move {
            let mut conn = Connection::connect().await?;
            let (_, mut event_listener) = conn.send(Request::EventStream).await?;

            loop {
                match event_listener() {
                    Ok(event) => {
                        #[cfg(feature = "workspaces+niri")]
                        let event = ws_handler.handle(event);

                        #[cfg(feature = "keyboard+niri")]
                        let event = event.and_then(|event| kb_handler.handle(event));

                        let _ = event;
                    }
                    Err(err) => {
                        error!("{err:?}");
                        break;
                    }
                }
            }

            Ok::<(), std::io::Error>(())
        });

        instance
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
        let rx = self.ws_handler.tx.subscribe();

        let workspaces = lock!(self.ws_handler.workspaces).clone();
        if !workspaces.is_empty() {
            self.ws_handler
                .tx
                .send_expect(WorkspaceUpdate::Init(workspaces));
        }

        rx
    }
}

#[cfg(feature = "keyboard+niri")]
impl KeyboardLayoutClient for Client {
    fn set_next_active(&self) {
        spawn(async move {
            let mut conn = Connection::connect().await?;

            let command = Request::Action(Action::SwitchLayout {
                layout: LayoutSwitchTarget::Next,
            });

            if let Err(err) = conn.send(command).await {
                error!("failed to send command: {err:?}");
            }
            Ok::<(), std::io::Error>(())
        });
    }

    fn subscribe(&self) -> broadcast::Receiver<KeyboardLayoutUpdate> {
        let rx = self.kb_handler.tx.subscribe();

        let update = lock!(self.kb_handler.layouts)
            .current()
            .map(|layout| KeyboardLayoutUpdate(layout.to_string()));

        if let Some(update) = update {
            self.kb_handler.tx.send_expect(update);
        } else {
            error!("Failed to get current keyboard layout");
        }

        rx
    }
}
