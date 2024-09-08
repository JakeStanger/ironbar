use core::str;
use serde::{Deserialize, Serialize};
use std::{env, io, path::Path};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use std::str::FromStr;

use crate::await_sync;

use super::compositor::{self, Visibility};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Request {
    Workspaces,
    Action(Action),
    EventStream,
}

pub type Reply = Result<Response, String>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Response {
    Handled,
    Workspaces(Vec<Workspace>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    FocusWorkspace { reference: WorkspaceReferenceArg },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceReferenceArg {
    Name(String),
    Index(u8),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Workspace {
    pub id: u64,
    pub name: Option<String>,
    pub output: Option<String>,
    pub is_active: bool,
    pub is_focused: bool,
}

impl From<&Workspace> for compositor::Workspace {
    fn from(workspace: &Workspace) -> compositor::Workspace {
        compositor::Workspace {
            id: workspace.id as i64,
            name: workspace.name.clone().unwrap_or(workspace.id.to_string()),
            monitor: workspace.output.clone().unwrap_or_default(),
            visibility: match workspace.is_focused {
                true => Visibility::focused(),
                false => match workspace.is_active {
                    true => Visibility::visible(),
                    false => Visibility::Hidden,
                },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    WorkspacesChanged { workspaces: Vec<Workspace> },
    WorkspaceActivated { id: u64, focused: bool },
    Other,
}

impl FromStr for WorkspaceReferenceArg {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let reference = if let Ok(index) = s.parse::<i32>() {
            if let Ok(idx) = u8::try_from(index) {
                Self::Index(idx)
            } else {
                return Err("workspace index must be between 0 and 255");
            }
        } else {
            Self::Name(s.to_string())
        };

        Ok(reference)
    }
}

#[derive(Debug)]
pub struct Connection(UnixStream);
impl Connection {
    pub async fn connect() -> io::Result<Self> {
        let socket_path = env::var_os("NIRI_SOCKET")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "NIRI_SOCKET not found!"))?;
        Self::connect_to(socket_path).await
    }

    pub async fn connect_to(path: impl AsRef<Path>) -> io::Result<Self> {
        let raw_stream = UnixStream::connect(path.as_ref()).await?;
        let stream = raw_stream;
        Ok(Self(stream))
    }

    pub async fn send<'a>(
        &'a mut self,
        request: Request,
    ) -> io::Result<(Reply, impl FnMut() -> io::Result<Event> + 'a)> {
        let Self(stream) = self;
        let mut buf = serde_json::to_string(&request).unwrap();
        stream.write_all(buf.as_bytes()).await?;
        stream.shutdown().await?;

        buf.clear();
        let mut reader = BufReader::new(stream);
        reader.read_line(&mut buf).await?;
        let reply = serde_json::from_str(&buf)?;

        let events = move || {
            buf.clear();
            await_sync(async {
                reader.read_line(&mut buf).await.unwrap();
            });
            let event: Event = serde_json::from_str(&buf).unwrap_or(Event::Other);
            Ok(event)
        };
        Ok((reply, events))
    }
}
