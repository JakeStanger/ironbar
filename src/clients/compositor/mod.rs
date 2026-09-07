use crate::register_fallible_client;
use cfg_if::cfg_if;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::debug;

#[cfg(feature = "hyprland")]
pub mod hyprland;
#[cfg(feature = "mangowm")]
pub mod mangowm;
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
    #[cfg(feature = "mangowm")]
    MangoWm,
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
                #[cfg(feature = "workspaces+mangowm")]
                Self::MangoWm => "MangoWm",
                Self::Unsupported => "Unsupported",
            }
        )
    }
}

impl Compositor {
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
        } else if std::env::var("MANGO_INSTANCE_SIGNATURE").is_ok() {
            cfg_if! {
                if #[cfg(feature = "mangowm")] { Self::MangoWm }
                else { tracing::error!("Not compiled with MangoWm support"); Self::Unsupported }
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
            #[cfg(feature = "workspaces+mangowm")]
            Self::MangoWm => Ok(Arc::new(mangowm::Client::new())),
            Self::Unsupported => Err(Error::Unsupported(
                "workspaces",
                &["sway", "hyprland", "niri", "mangowm"],
            )),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Disabled("workspaces")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i64,
    pub index: i64,
    pub name: String,
    pub monitor: String,
    pub visibility: Visibility,
}

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
    Init(Vec<Workspace>),
    Add(Workspace),
    Remove(i64),
    Move(Workspace),
    Focus {
        old: Option<Workspace>,
        new: Workspace,
    },

    Rename {
        id: i64,
        name: String,
    },

    Urgent {
        id: i64,
        urgent: bool,
    },

    Unknown,
}

#[derive(Clone, Debug)]
#[cfg(feature = "bindmode")]
pub struct BindModeUpdate {
    pub name: String,
    pub pango_markup: bool,
}

#[cfg(feature = "workspaces")]
pub trait WorkspaceClient: Debug + Send + Sync {
    fn focus(&self, id: i64);

    fn subscribe(&self) -> broadcast::Receiver<WorkspaceUpdate>;
}

#[cfg(feature = "workspaces")]
register_fallible_client!(dyn WorkspaceClient, workspaces);

#[cfg(feature = "keyboard")]
pub trait KeyboardLayoutClient: Debug + Send + Sync {
    fn set_next_active(&self);

    fn subscribe(&self) -> broadcast::Receiver<KeyboardLayoutUpdate>;
}

#[cfg(feature = "keyboard")]
register_fallible_client!(dyn KeyboardLayoutClient, keyboard_layout);

#[cfg(feature = "bindmode")]
pub trait BindModeClient: Debug + Send + Sync {
    fn subscribe(&self) -> Result<broadcast::Receiver<BindModeUpdate>>;
}

#[cfg(feature = "bindmode")]
register_fallible_client!(dyn BindModeClient, bindmode);
