use super::{Visibility, Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::channels::SyncSenderExt;
use crate::spawn;
use serde::Deserialize;
use std::process::Stdio;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast::{Receiver, Sender, channel};
use tracing::{debug, error};

#[derive(Debug, Deserialize)]
struct Tag {
    index: i64,
    is_active: bool,
    #[serde(default)]
    client_count: i64,
}

#[derive(Debug, Deserialize)]
struct MonitorTags {
    monitor: String,
    tags: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
struct AllTags {
    all_tags: Vec<MonitorTags>,
}

fn make_id(monitor_index: i64, tag_index: i64) -> i64 {
    monitor_index * 100 + tag_index
}

fn to_workspaces(all_tags: AllTags) -> Vec<Workspace> {
    let mut out = Vec::new();

    for (mon_idx, mon) in all_tags.all_tags.into_iter().enumerate() {
        for tag in mon.tags {
            if !tag.is_active && tag.client_count == 0 {
                continue;
            }

            out.push(Workspace {
                id: make_id(mon_idx as i64, tag.index),
                index: tag.index,
                name: tag.index.to_string(),
                monitor: mon.monitor.clone(),
                visibility: if tag.is_active {
                    Visibility::focused()
                } else {
                    Visibility::visible()
                },
            });
        }
    }

    out
}

#[derive(Debug)]
pub struct Client {
    tx: Sender<WorkspaceUpdate>,
    _rx: Receiver<WorkspaceUpdate>,
    workspaces: Arc<RwLock<Vec<Workspace>>>,
}

impl Client {
    pub fn new() -> Self {
        let (tx, rx) = channel(16);
        let tx2 = tx.clone();

        let workspaces = Arc::new(RwLock::new(Vec::new()));
        let workspaces2 = workspaces.clone();

        spawn(async move {
            loop {
                if let Err(err) = watch_loop(&tx, &workspaces).await {
                    error!("mangowm: `mmsg watch all-tags` error: {err:#}, retrying in 2s");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        });

        Self {
            tx: tx2,
            _rx: rx,
            workspaces: workspaces2,
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

async fn watch_loop(
    tx: &Sender<WorkspaceUpdate>,
    workspaces: &Arc<RwLock<Vec<Workspace>>>,
) -> color_eyre::Result<()> {
    let mut child = Command::new("mmsg")
        .args(["watch", "all-tags"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| color_eyre::Report::msg("mmsg watch all-tags: no stdout"))?;

    let mut lines = BufReader::new(stdout).lines();
    let mut first = true;

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let all_tags: AllTags = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(err) => {
                error!("mangowm: failed to parse `mmsg watch all-tags` line: {err} ({line})");
                continue;
            }
        };

        let new_workspaces = to_workspaces(all_tags);
        debug!("mangowm: all-tags update: {new_workspaces:?}");

        if first {
            first = false;
            tx.send_expect(WorkspaceUpdate::Init(new_workspaces.clone()));
        } else {
            let old_workspaces = workspaces.read().expect("lock poisoned").clone();
            for update in diff(&old_workspaces, &new_workspaces) {
                tx.send_expect(update);
            }
        }

        *workspaces.write().expect("lock poisoned") = new_workspaces;
    }

    let status = child.wait().await.ok();
    Err(color_eyre::Report::msg(format!(
        "mmsg watch all-tags exited: {status:?}"
    )))
}

fn diff(old: &[Workspace], new: &[Workspace]) -> Vec<WorkspaceUpdate> {
    let mut updates = Vec::new();

    for workspace in new {
        match old.iter().find(|w| w.id == workspace.id) {
            None => updates.push(WorkspaceUpdate::Add(workspace.clone())),
            Some(old_workspace) if old_workspace.monitor != workspace.monitor => {
                updates.push(WorkspaceUpdate::Move(workspace.clone()));
            }
            _ => {}
        }
    }

    for workspace in old {
        if !new.iter().any(|w| w.id == workspace.id) {
            updates.push(WorkspaceUpdate::Remove(workspace.id));
        }
    }

    for new_focused in new.iter().filter(|w| w.visibility.is_focused()) {
        let old_focused = old
            .iter()
            .find(|w| w.monitor == new_focused.monitor && w.visibility.is_focused());

        if old_focused.map(|w| w.id) != Some(new_focused.id) {
            updates.push(WorkspaceUpdate::Focus {
                old: old_focused.cloned(),
                new: new_focused.clone(),
            });
        }
    }

    updates
}

#[cfg(feature = "workspaces+mangowm")]
impl WorkspaceClient for Client {
    fn focus(&self, id: i64) {
        let tag_index = id % 100;

        spawn(async move {
            let arg = format!("view,{tag_index},0");
            match Command::new("mmsg")
                .args(["dispatch", &arg])
                .stdin(Stdio::null())
                .output()
                .await
            {
                Ok(output) if !output.status.success() => {
                    error!(
                        "mangowm: `mmsg dispatch {arg}` failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains("\"error\"") {
                        error!("mangowm: `mmsg dispatch {arg}` returned: {stdout}");
                    }
                }
                Err(err) => error!("mangowm: failed to run `mmsg dispatch {arg}`: {err}"),
            }
        });
    }

    fn subscribe(&self) -> Receiver<WorkspaceUpdate> {
        let rx = self.tx.subscribe();

        let workspaces = self.workspaces.read().expect("lock poisoned");
        if !workspaces.is_empty() {
            self.tx
                .send_expect(WorkspaceUpdate::Init(workspaces.clone()));
        }

        rx
    }
}
