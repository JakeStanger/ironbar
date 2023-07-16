mod client;
pub mod commands;
pub mod responses;
mod server;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tracing::warn;

use crate::GlobalState;
pub use commands::Command;
pub use responses::Response;

#[derive(Debug)]
pub struct Ipc {
    path: PathBuf,
    global_state: Rc<RefCell<GlobalState>>,
}

impl Ipc {
    /// Creates a new IPC instance.
    /// This can be used as both a server and client.
    pub fn new(global_state: Rc<RefCell<GlobalState>>) -> Self {
        let ipc_socket_file = std::env::var("XDG_RUNTIME_DIR")
            .map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from)
            .join("ironbar-ipc.sock");

        if format!("{}", ipc_socket_file.display()).len() > 100 {
            warn!("The IPC socket file's absolute path exceeds 100 bytes, the socket may fail to create.");
        }

        Self {
            path: ipc_socket_file,
            global_state,
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}
