mod bar;
mod ironvar;
mod style;

use std::fs;
use std::path::Path;
use std::rc::Rc;

use color_eyre::{Report, Result};
use gtk::Application;
use gtk::prelude::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{debug, error, info, trace, warn};

use super::Ipc;
use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::ipc::{Command, Response};
use crate::{Ironbar, spawn};

impl Ipc {
    /// Starts the IPC server on its socket.
    ///
    /// Once started, the server will begin accepting connections.
    pub fn start(&self, application: &Application, ironbar: Rc<Ironbar>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (res_tx, mut res_rx) = mpsc::channel(32);

        let path = self.path.clone();

        if path.exists() {
            warn!("Socket already exists. Did Ironbar exit abruptly?");
            warn!("Attempting IPC shutdown to allow binding to address");
            Self::shutdown(&path);
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
                        debug!("handling incoming connection");
                        if let Err(err) =
                            Self::handle_connection(stream, &cmd_tx, &mut res_rx).await
                        {
                            error!("{err:?}");
                        }
                        debug!("done");
                    }
                    Err(err) => {
                        error!("{err:?}");
                    }
                }
            }
        });

        cmd_rx.recv_glib(application, move |application, command| {
            let res = Self::handle_command(command, application, &ironbar);
            res_tx.send_spawn(res);
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
        trace!("awaiting readable state");
        stream.readable().await?;

        let mut read_buffer = Vec::with_capacity(1024);

        let mut reader = BufReader::new(&mut stream);

        trace!("reading bytes");
        let bytes = reader.read_until(b'\n', &mut read_buffer).await?;
        debug!("read {} bytes", bytes);

        // FIXME: Error on invalid command
        let command = serde_json::from_slice::<Command>(&read_buffer[..bytes])?;

        debug!("Received command: {command:?}");

        cmd_tx.send_expect(command).await;
        let res = res_rx
            .recv()
            .await
            .unwrap_or(Response::Err { message: None });

        let mut res = serde_json::to_vec(&res)?;
        res.push(b'\n');

        trace!("awaiting writable state");
        stream.writable().await?;

        debug!("writing {} bytes", res.len());
        stream.write_all(&res).await?;

        trace!("bytes written, shutting down stream");
        stream.shutdown().await?;

        Ok(())
    }

    /// Takes an input command, runs it and returns with the appropriate response.
    ///
    /// This runs on the main thread, allowing commands to interact with GTK.
    fn handle_command(
        command: Command,
        application: &Application,
        ironbar: &Rc<Ironbar>,
    ) -> Response {
        match command {
            Command::Ping => Response::Ok,
            Command::Inspect => {
                gtk::Window::set_interactive_debugging(true);
                Response::Ok
            }
            Command::Reload => {
                info!("Closing existing bars");
                ironbar.bars.borrow_mut().clear();

                let windows = application.windows();
                for window in windows {
                    window.close();
                }

                ironbar.reload_config();

                crate::load_output_bars(ironbar, application);
                Response::Ok
            }
            Command::Var(cmd) => ironvar::handle_command(cmd),
            Command::Bar(cmd) => bar::handle_command(&cmd, ironbar),
            Command::Style(cmd) => style::handle_command(cmd, ironbar),
        }
    }

    /// Shuts down the IPC server,
    /// removing the socket file in the process.
    ///
    /// Note this is static as the `Ipc` struct is not `Send`.
    pub fn shutdown<P: AsRef<Path>>(path: P) {
        fs::remove_file(&path).ok();
    }
}
