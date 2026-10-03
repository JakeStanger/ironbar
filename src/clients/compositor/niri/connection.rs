/// Taken from the `niri_ipc` crate.
/// Only a relevant snippet has been extracted
/// to reduce compile times.
use crate::await_sync;
use core::str;
use serde::{Deserialize, Serialize};
use std::io::Result;
use std::{env, path::Path};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Request {
    Action(Action),
    EventStream,
}

pub type Reply = std::result::Result<Response, String>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Response {
    Handled,
    #[cfg(feature = "workspaces")]
    Workspaces(Vec<ws::Workspace>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    #[cfg(feature = "workspaces")]
    FocusWorkspace {
        reference: ws::WorkspaceReferenceArg,
    },
    #[cfg(feature = "keyboard")]
    SwitchLayout { layout: kb::LayoutSwitchTarget },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    #[cfg(feature = "workspaces")]
    WorkspacesChanged {
        workspaces: Vec<ws::Workspace>,
    },
    #[cfg(feature = "workspaces")]
    WorkspaceActivated {
        id: u64,
        focused: bool,
    },
    #[cfg(feature = "workspaces")]
    WorkspaceUrgencyChanged {
        id: u64,
        urgent: bool,
    },
    #[cfg(feature = "keyboard")]
    KeyboardLayoutsChanged {
        keyboard_layouts: kb::KeyboardLayouts,
    },
    #[cfg(feature = "keyboard")]
    KeyboardLayoutSwitched {
        idx: u8,
    },
    Other,
}

#[cfg(feature = "keyboard")]
pub mod kb {
    use super::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone, Copy)]
    pub enum LayoutSwitchTarget {
        Next,
        Prev,
        Index(u8),
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    pub struct KeyboardLayouts {
        pub names: Vec<String>,
        pub current_idx: u8,
    }

    impl KeyboardLayouts {
        pub fn current(&self) -> Option<&str> {
            self.names
                .get(self.current_idx as usize)
                .map(|x| x.as_str())
        }
    }
}

#[cfg(feature = "workspaces")]
pub mod ws {
    use super::{Deserialize, Serialize};
    use crate::clients::compositor::{Visibility, Workspace as IronWorkspace};

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub enum WorkspaceReferenceArg {
        Name(String),
        Id(u64),
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Workspace {
        pub id: u64,
        pub idx: u8,
        pub name: Option<String>,
        pub output: Option<String>,
        pub is_active: bool,
        pub is_focused: bool,
    }

    impl From<&Workspace> for IronWorkspace {
        fn from(workspace: &Workspace) -> IronWorkspace {
            // Workspaces in niri don't neccessarily have names.
            // If the niri workspace has a name then it is assigned as is,
            // but if it does not have a name, the monitor index is used.
            Self {
                id: workspace.id as i64,
                index: workspace.idx as i64,
                name: workspace.name.clone().unwrap_or(workspace.idx.to_string()),
                monitor: workspace.output.clone().unwrap_or_default(),
                visibility: if workspace.is_active {
                    Visibility::Visible {
                        focused: workspace.is_focused,
                    }
                } else {
                    Visibility::Hidden
                },
            }
        }
    }
}

#[derive(Debug)]
pub struct Connection(UnixStream);
impl Connection {
    pub async fn connect() -> Result<Self> {
        let socket_path = env::var_os("NIRI_SOCKET").ok_or_else(|| {
            // technically this isn't really an io error, but it's close enough
            std::io::Error::new(std::io::ErrorKind::NotFound, "NIRI_SOCKET not found")
        })?;
        Self::connect_to(socket_path).await
    }

    pub async fn connect_to(path: impl AsRef<Path>) -> Result<Self> {
        let raw_stream = UnixStream::connect(path.as_ref()).await?;
        let stream = raw_stream;
        Ok(Self(stream))
    }

    pub async fn send(
        &mut self,
        request: Request,
    ) -> Result<(Reply, impl FnMut() -> Result<Event> + '_)> {
        let Self(stream) = self;
        let mut buf = serde_json::to_string(&request)?;

        stream.write_all(buf.as_bytes()).await?;
        stream.shutdown().await?;

        buf.clear();
        let mut reader = BufReader::new(stream);
        reader.read_line(&mut buf).await?;
        let reply = serde_json::from_str(&buf)?;

        let events = move || {
            buf.clear();
            await_sync(async {
                reader.read_line(&mut buf).await.unwrap_or(0);
            });
            let event: Event = serde_json::from_str(&buf).unwrap_or(Event::Other);
            Ok(event)
        };
        Ok((reply, events))
    }
}
