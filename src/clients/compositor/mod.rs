use color_eyre::{Help, Report, Result};
use std::fmt::{Display, Formatter};
use tokio::sync::broadcast;
use tracing::debug;

pub mod hyprland;
pub mod sway;

pub enum Compositor {
    Sway,
    Hyprland,
    Unsupported,
}

impl Display for Compositor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Compositor::Sway => "Sway",
                Compositor::Hyprland => "Hyprland",
                Compositor::Unsupported => "Unsupported",
            }
        )
    }
}

impl Compositor {
    /// Attempts to get the current compositor.
    /// This is done by checking system env vars.
    fn get_current() -> Self {
        if std::env::var("SWAYSOCK").is_ok() {
            Self::Sway
        } else if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            Self::Hyprland
        } else {
            Self::Unsupported
        }
    }

    /// Gets the workspace client for the current compositor
    pub fn get_workspace_client() -> Result<&'static (dyn WorkspaceClient + Send)> {
        let current = Self::get_current();
        debug!("Getting workspace client for: {current}");
        match current {
            Compositor::Sway => Ok(sway::get_sub_client()),
            Compositor::Hyprland => Ok(hyprland::get_client()),
            Compositor::Unsupported => Err(Report::msg("Unsupported compositor")
                .note("Currently workspaces are only supported by Sway and Hyprland")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    /// Unique identifier
    pub id: String,
    /// Workspace friendly name
    pub name: String,
    /// Name of the monitor (output) the workspace is located on
    pub monitor: String,
    /// Whether the workspace is in focus
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub enum WorkspaceUpdate {
    /// Provides an initial list of workspaces.
    /// This is re-sent to all subscribers when a new subscription is created.
    Init(Vec<Workspace>),
    Add(Workspace),
    Remove(Workspace),
    Update(Workspace),
    Move(Workspace),
    /// Declares focus moved from the old workspace to the new.
    Focus {
        old: Workspace,
        new: Workspace,
    },
}

pub trait WorkspaceClient {
    /// Requests the workspace with this name is focused.
    fn focus(&self, name: String) -> Result<()>;

    /// Creates a new to workspace event receiver.
    fn subscribe_workspace_change(&self) -> broadcast::Receiver<WorkspaceUpdate>;
}
