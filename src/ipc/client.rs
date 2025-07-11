use super::Ipc;
use crate::ipc::{Command, Response};
use color_eyre::Result;
use color_eyre::{Help, Report};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

impl Ipc {
    /// Sends a command to the IPC server.
    /// The server response is returned.
    pub async fn send(&self, command: Command, debug: bool) -> Result<Response> {
        let mut stream = match UnixStream::connect(&self.path).await {
            Ok(stream) => Ok(stream),
            Err(err) => Err(Report::new(err)
                .wrap_err("Failed to connect to Ironbar IPC server")
                .suggestion("Is Ironbar running?")),
        }?;

        let mut write_buffer = serde_json::to_vec(&command)?;

        if debug {
            eprintln!("REQUEST JSON: {}", serde_json::to_string(&command)?);
        }

        write_buffer.push(b'\n');
        stream.write_all(&write_buffer).await?;

        let mut read_buffer = String::new();
        let mut reader = BufReader::new(stream);
        let bytes = reader.read_line(&mut read_buffer).await?;

        let response = serde_json::from_str(&read_buffer[..bytes])?;
        Ok(response)
    }
}
