use super::Ipc;
use crate::bridge_channel::BridgeChannel;
use crate::ipc::{Command, Response};
use crate::ironvar::get_variable_manager;
use crate::style::load_css;
use crate::{read_lock, send_async, try_send, write_lock};
use color_eyre::{Report, Result};
use glib::Continue;
use std::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, error, info, warn};

impl Ipc {
    /// Starts the IPC server on its socket.
    ///
    /// Once started, the server will begin accepting connections.
    pub fn start(&self) {
        let bridge = BridgeChannel::<Command>::new();
        let cmd_tx = bridge.create_sender();
        let (res_tx, mut res_rx) = mpsc::channel(32);

        let path = self.path.clone();

        if path.exists() {
            warn!("Socket already exists. Did Ironbar exit abruptly?");
            warn!("Attempting IPC shutdown to allow binding to address");
            self.shutdown();
        }

        spawn(async move {
            info!("Starting IPC on {}", path.display());

            let listener = match UnixListener::bind(&path) {
                Ok(listener) => listener,
                Err(err) => {
                    error!(
                        "{:?}",
                        Report::new(err).wrap_err("Unable to start IPC server")
                    );
                    return;
                }
            };

            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        if let Err(err) =
                            Self::handle_connection(stream, &cmd_tx, &mut res_rx).await
                        {
                            error!("{err:?}");
                        }
                    }
                    Err(err) => {
                        error!("{err:?}");
                    }
                }
            }
        });

        bridge.recv(move |command| {
            let res = Self::handle_command(command);
            try_send!(res_tx, res);
            Continue(true)
        });
    }

    /// Takes an incoming connections,
    /// reads the command message, and sends the response.
    ///
    /// The connection is closed once the response has been written.
    async fn handle_connection(
        mut stream: UnixStream,
        cmd_tx: &Sender<Command>,
        res_rx: &mut Receiver<Response>,
    ) -> Result<()> {
        let (mut stream_read, mut stream_write) = stream.split();

        let mut read_buffer = vec![0; 1024];
        let bytes = stream_read.read(&mut read_buffer).await?;

        let command = serde_json::from_slice::<Command>(&read_buffer[..bytes])?;

        debug!("Received command: {command:?}");

        send_async!(cmd_tx, command);
        let res = res_rx
            .recv()
            .await
            .unwrap_or(Response::Err { message: None });
        let res = serde_json::to_vec(&res)?;

        stream_write.write_all(&res).await?;
        stream_write.shutdown().await?;

        Ok(())
    }

    /// Takes an input command, runs it and returns with the appropriate response.
    ///
    /// This runs on the main thread, allowing commands to interact with GTK.
    fn handle_command(command: Command) -> Response {
        match command {
            Command::Inspect => {
                gtk::Window::set_interactive_debugging(true);
                Response::Ok
            }
            Command::Set { key, value } => {
                let variable_manager = get_variable_manager();
                let mut variable_manager = write_lock!(variable_manager);
                match variable_manager.set(key, value) {
                    Ok(_) => Response::Ok,
                    Err(err) => Response::error(&format!("{err}")),
                }
            }
            Command::Get { key } => {
                let variable_manager = get_variable_manager();
                let value = read_lock!(variable_manager).get(&key);
                match value {
                    Some(value) => Response::OkValue { value },
                    None => Response::error("Variable not found"),
                }
            }
            Command::LoadCss { path } => {
                if path.exists() {
                    load_css(path);
                    Response::Ok
                } else {
                    Response::error("File not found")
                }
            }
            Command::Ping => Response::Ok,
        }
    }

    /// Shuts down the IPC server,
    /// removing the socket file in the process.
    pub fn shutdown(&self) {
        fs::remove_file(&self.path).ok();
    }
}
