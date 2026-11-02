use ironbar_ipc_types::{Command, Ipc, Response};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

pub trait IpcClient {
    /// Sends a command to the IPC server.
    /// The server response is returned.
    fn send(&self, command: Command, debug: bool) -> io::Result<Response>;
}

impl IpcClient for Ipc {
    fn send(&self, command: Command, debug: bool) -> io::Result<Response> {
        let mut stream = UnixStream::connect(self.path()).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "Failed to connect to Ironbar IPC server at '{}': {err}\nIs Ironbar running?",
                    self.path().display()
                ),
            )
        })?;

        let mut write_buffer = serde_json::to_vec(&command)?;

        if debug {
            eprintln!("REQUEST JSON: {}", String::from_utf8_lossy(&write_buffer));
        }

        write_buffer.push(b'\n');
        stream.write_all(&write_buffer)?;

        let mut read_buffer = String::new();
        let bytes = BufReader::new(stream).read_line(&mut read_buffer)?;

        let response = serde_json::from_str(&read_buffer[..bytes])?;
        Ok(response)
    }
}
