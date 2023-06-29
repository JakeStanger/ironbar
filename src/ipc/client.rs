use super::Ipc;
use crate::ipc::{Command, Response};
use color_eyre::Result;
use color_eyre::{Help, Report};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

impl Ipc {
    /// Sends a command to the IPC server.
    /// The server response is returned.
    pub async fn send(&self, command: Command) -> Result<Response> {
        let mut stream = match UnixStream::connect(&self.path).await {
            Ok(stream) => Ok(stream),
            Err(err) => Err(Report::new(err)
                .wrap_err("Failed to connect to Ironbar IPC server")
                .suggestion("Is Ironbar running?")),
        }?;

        let write_buffer = serde_json::to_vec(&command)?;
        stream.write_all(&write_buffer).await?;

        let mut read_buffer = vec![0; 1024];
        let bytes = stream.read(&mut read_buffer).await?;

        let response = serde_json::from_slice(&read_buffer[..bytes])?;
        Ok(response)
    }
}
