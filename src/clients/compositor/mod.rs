use cfg_if::cfg_if;
use color_eyre::{Help, Report, Result};
use std::fmt::{Display, Formatter};
use tokio::sync::broadcast;
use tracing::debug;

#[cfg(feature = "workspaces+hyprland")]
pub mod hyprland;
#[cfg(feature = "workspaces+sway")]
pub mod sway;

pub enum Compositor {
    #[cfg(feature = "workspaces+sway")]
    Sway,
    #[cfg(feature = "workspaces+hyprland")]
    Hyprland,
    Unsupported,
}

impl Display for Compositor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                #[cfg(feature = "workspaces+sway")]
                Self::Sway => "Sway",
                #[cfg(feature = "workspaces+hyprland")]
                Self::Hyprland => "Hyprland",
                Self::Unsupported => "Unsupported",
            }
        )
    }
}

impl Compositor {
    /// Attempts to get the current compositor.
    /// This is done by checking system env vars.
    fn get_current() -> Self {
        if std::env::var("SWAYSOCK").is_ok() {
            cfg_if! {
                if #[cfg(feature = "workspaces+sway")] { Self::Sway }
                else { tracing::error!("Not compiled with Sway support"); Self::Unsupported }
            }
        } else if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            cfg_if! {
                if #[cfg(feature = "workspaces+hyprland")] { Self::Hyprland}
                else { tracing::error!("Not compiled with Hyprland support"); Self::Unsupported }
            }
        } else {
            Self::Unsupported
        }
    }

    /// Gets the workspace client for the current compositor
    pub fn get_workspace_client() -> Result<&'static (dyn WorkspaceClient + Send)> {
        let current = Self::get_current();
        debug!("Getting workspace client for: {current}");
        match current {
            #[cfg(feature = "workspaces+sway")]
            Self::Sway => Ok(sway::get_sub_client()),
            #[cfg(feature = "workspaces+hyprland")]
            Self::Hyprland => Ok(hyprland::get_client()),
            Self::Unsupported => Err(Report::msg("Unsupported compositor")
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
    /// How visible the workspace is
    pub visibility: Visibility,
}

/// Indicates workspace visibility. Visible workspaces have a boolean flag to indicate if they are also focused.
/// Yes, this is the same signature as Option<bool>, but it's impl is a lot more suited for our case.
#[derive(Debug, Copy, Clone)]
pub enum Visibility {
    Visible(bool),
    Hidden,
}

impl Visibility {
    pub fn visible() -> Self {
        Self::Visible(false)
    }

    pub fn focused() -> Self {
        Self::Visible(true)
    }

    pub fn is_visible(self) -> bool {
        matches!(self, Self::Visible(_))
    }

    pub fn is_focused(self) -> bool {
        if let Self::Visible(focused) = self {
            focused
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub enum WorkspaceUpdate {
    /// Provides an initial list of workspaces.
    /// This is re-sent to all subscribers when a new subscription is created.
    Init(Vec<Workspace>),
    Add(Workspace),
    Remove(String),
    Update(Workspace),
    Move(Workspace),
    /// Declares focus moved from the old workspace to the new.
    Focus {
        old: Option<Workspace>,
        new: Workspace,
    },
}

pub trait WorkspaceClient {
    /// Requests the workspace with this name is focused.
    fn focus(&self, name: String) -> Result<()>;

    /// Creates a new to workspace event receiver.
    fn subscribe_workspace_change(&self) -> broadcast::Receiver<WorkspaceUpdate>;
}
