mod commands;
mod responses;

pub use commands::*;
pub use responses::*;

use std::path::{Path, PathBuf};

/// Logs a warning via `tracing` if enabled, otherwise to stderr.
macro_rules! warn {
    ($($arg:tt)*) => {{
        #[cfg(feature = "tracing")]
        ::tracing::warn!($($arg)*);
        #[cfg(not(feature = "tracing"))]
        ::std::eprintln!("WARN: {}", ::std::format_args!($($arg)*));
    }};
}

#[derive(Debug)]
pub struct Ipc {
    path: PathBuf,
}

impl Ipc {
    /// Creates a new IPC instance.
    /// This can be used as both a server and client.
    pub fn new() -> Self {
        let ipc_socket_file = std::env::var("XDG_RUNTIME_DIR")
            .map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from)
            .join("ironbar-ipc.sock");

        if format!("{}", ipc_socket_file.display()).len() > 100 {
            warn!(
                "The IPC socket file's absolute path exceeds 100 bytes, the socket may fail to create."
            );
        }

        Self {
            path: ipc_socket_file,
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

impl Default for Ipc {
    fn default() -> Self {
        Self::new()
    }
}
