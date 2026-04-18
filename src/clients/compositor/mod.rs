use crate::register_fallible_client;
use cfg_if::cfg_if;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::debug;

#[cfg(feature = "hyprland")]
pub mod hyprland;
#[cfg(feature = "niri")]
pub mod niri;
#[cfg(feature = "sway")]
pub mod sway;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0} is unsupported by compositor. The following are supported: {1:?}")]
    Unsupported(&'static str, &'static [&'static str]),
    #[error("{0} feature flag is disabled for compositor")]
    Disabled(&'static str),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, Error>;

pub enum Compositor {
    #[cfg(feature = "sway")]
    Sway,
    #[cfg(feature = "hyprland")]
    Hyprland,
    #[cfg(feature = "niri")]
    Niri,
    Unsupported,
}

impl Display for Compositor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                #[cfg(any(feature = "sway"))]
                Self::Sway => "Sway",
                #[cfg(any(feature = "hyprland"))]
                Self::Hyprland => "Hyprland",
                #[cfg(feature = "workspaces+niri")]
                Self::Niri => "Niri",
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
                if #[cfg(feature = "sway")] { Self::Sway }
                else { tracing::error!("Not compiled with Sway support"); Self::Unsupported }
            }
        } else if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            cfg_if! {
                if #[cfg(feature = "hyprland")] { Self::Hyprland }
                else { tracing::error!("Not compiled with Hyprland support"); Self::Unsupported }
            }
        } else if std::env::var("NIRI_SOCKET").is_ok() {
            cfg_if! {
                if #[cfg(feature = "niri")] { Self::Niri }
                else {tracing::error!("Not compiled with Niri support"); Self::Unsupported }
            }
        } else {
            Self::Unsupported
        }
    }

    #[cfg(feature = "bindmode")]
    pub fn create_bindmode_client(
        clients: &mut super::Clients,
    ) -> Result<Arc<dyn BindModeClient + Send + Sync>> {
        let current = Self::get_current();
        debug!("Getting keyboard_layout client for: {current}");
        match current {
            #[cfg(feature = "bindmode+sway")]
            Self::Sway => Ok(clients.sway().map_err(|err| Error::Other(err.into()))?),
            #[cfg(feature = "bindmode+hyprland")]
            Self::Hyprland => Ok(clients.hyprland()),
            #[cfg(feature = "niri")]
            Self::Niri => Err(Error::Unsupported("bindmode", &["sway", "hyprland"])),
            Self::Unsupported => Err(Error::Unsupported("bindmode", &["sway", "hyprland"])),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Disabled("bindmode")),
        }
    }

    #[cfg(feature = "keyboard")]
    pub fn create_keyboard_layout_client(
        clients: &mut super::Clients,
    ) -> Result<Arc<dyn KeyboardLayoutClient + Send + Sync>> {
        let current = Self::get_current();
        debug!("Getting keyboard_layout client for: {current}");
        match current {
            #[cfg(feature = "keyboard+sway")]
            Self::Sway => Ok(clients.sway().map_err(|err| Error::Other(err.into()))?),
            #[cfg(feature = "keyboard+hyprland")]
            Self::Hyprland => Ok(clients.hyprland()),
            #[cfg(feature = "niri")]
            Self::Niri => Err(Error::Unsupported("keyboard", &["sway", "hyprland"])),
            Self::Unsupported => Err(Error::Unsupported("keyboard", &["sway", "hyprland"])),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Disabled("keyboard")),
        }
    }

    /// Creates a new instance of
    /// the workspace client for the current compositor.
    #[cfg(feature = "workspaces")]
    pub fn create_workspace_client(
        clients: &mut super::Clients,
    ) -> Result<Arc<dyn WorkspaceClient + Send + Sync>> {
        let current = Self::get_current();
        debug!("Getting workspace client for: {current}");
        match current {
            #[cfg(feature = "workspaces+sway")]
            Self::Sway => Ok(clients.sway().map_err(|err| Error::Other(err.into()))?),
            #[cfg(feature = "workspaces+hyprland")]
            Self::Hyprland => Ok(clients.hyprland()),
            #[cfg(feature = "workspaces+niri")]
            Self::Niri => Ok(Arc::new(niri::Client::new())),
            Self::Unsupported => Err(Error::Unsupported(
                "workspaces",
                &["sway", "hyprland", "niri"],
            )),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Disabled("workspaces")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    /// Unique identifier
    pub id: i64,
    /// The workspace index (e.g. for sorting)
    pub index: i64,
    /// Workspace friendly name
    pub name: String,
    /// Name of the monitor (output) the workspace is located on
    pub monitor: String,
    /// How visible the workspace is
    pub visibility: Visibility,
    /// How many windows does the workspace have
    pub windows: u16,
}

/// Indicates workspace visibility.
/// Visible workspaces have a boolean flag to indicate if they are also focused.
#[derive(Debug, Copy, Clone)]
pub enum Visibility {
    Visible { focused: bool },
    Hidden,
}

impl Visibility {
    pub fn visible() -> Self {
        Self::Visible { focused: false }
    }

    pub fn focused() -> Self {
        Self::Visible { focused: true }
    }

    pub fn is_visible(self) -> bool {
        matches!(self, Self::Visible { .. })
    }

    pub fn is_focused(self) -> bool {
        if let Self::Visible { focused } = self {
            focused
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
#[cfg(feature = "keyboard")]
pub struct KeyboardLayoutUpdate(pub String);

#[derive(Debug, Clone)]
#[cfg(feature = "workspaces")]
pub enum WorkspaceUpdate {
    /// Provides an initial list of workspaces.
    /// This is re-sent to all subscribers when a new subscription is created.
    Init(Vec<Workspace>),
    Add(Workspace),
    Remove(i64),
    Move(Workspace),
    /// Declares focus moved from the old workspace to the new.
    Focus {
        old: Option<Workspace>,
        new: Workspace,
    },

    Rename {
        id: i64,
        name: String,
    },

    /// The urgent state of a node changed.
    Urgent {
        id: i64,
        urgent: bool,
    },

    Windows {
        id: i64,
        windows: u16,
    },

    /// An update was triggered by the compositor but this was not mapped by Ironbar.
    ///
    /// This is purely used for ergonomics within the compositor clients
    /// and should be ignored by consumers.
    Unknown,
}

#[derive(Clone, Debug)]
#[cfg(feature = "bindmode")]
pub struct BindModeUpdate {
    /// The binding mode that became active.
    pub name: String,
    /// Whether the mode should be parsed as pango markup.
    pub pango_markup: bool,
}

#[cfg(feature = "workspaces")]
pub trait WorkspaceClient: Debug + Send + Sync {
    /// Requests the workspace with this id is focused.
    fn focus(&self, id: i64);

    /// Creates a new to workspace event receiver.
    fn subscribe(&self) -> broadcast::Receiver<WorkspaceUpdate>;
}

#[cfg(feature = "workspaces")]
register_fallible_client!(dyn WorkspaceClient, workspaces);

#[cfg(feature = "keyboard")]
pub trait KeyboardLayoutClient: Debug + Send + Sync {
    /// Switches to the next layout.
    fn set_next_active(&self);

    /// Creates a new to keyboard layout event receiver.
    fn subscribe(&self) -> broadcast::Receiver<KeyboardLayoutUpdate>;
}

#[cfg(feature = "keyboard")]
register_fallible_client!(dyn KeyboardLayoutClient, keyboard_layout);

#[cfg(feature = "bindmode")]
pub trait BindModeClient: Debug + Send + Sync {
    /// Add a callback for bindmode updates.
    fn subscribe(&self) -> Result<broadcast::Receiver<BindModeUpdate>>;
}

#[cfg(feature = "bindmode")]
register_fallible_client!(dyn BindModeClient, bindmode);
