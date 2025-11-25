use super::Ipc;
use crate::ipc::{Command, Response};
use miette::{IntoDiagnostic, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

impl Ipc {
    /// Sends a command to the IPC server.
    /// The server response is returned.
    pub async fn send(&self, command: Command, debug: bool) -> Result<Response> {
        let mut stream = match UnixStream::connect(&self.path).await.into_diagnostic() {
            Ok(stream) => Ok(stream),
            Err(err) => Err(err.wrap_err("Failed to connect to Ironbar IPC server")),
        }?;

        let mut write_buffer = serde_json::to_vec(&command).into_diagnostic()?;

        if debug {
            eprintln!(
                "REQUEST JSON: {}",
                serde_json::to_string(&command).into_diagnostic()?
            );
        }

        write_buffer.push(b'\n');
        stream.write_all(&write_buffer).await.into_diagnostic()?;

        let mut read_buffer = String::new();
        let mut reader = BufReader::new(stream);
        let bytes = reader.read_line(&mut read_buffer).await.into_diagnostic()?;

        let response = serde_json::from_str(&read_buffer[..bytes]).into_diagnostic()?;
        Ok(response)
    }
}
