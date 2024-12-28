mod bar;
mod ironvar;

use std::fs;
use std::path::Path;
use std::rc::Rc;

use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::Application;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{debug, error, info, warn};

use super::Ipc;
use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::ipc::{Command, Response};
use crate::style::load_css;
use crate::{spawn, Ironbar};

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

        let application = application.clone();
        cmd_rx.recv_glib(move |command| {
            let res = Self::handle_command(command, &application, &ironbar);
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
        let (mut stream_read, mut stream_write) = stream.split();

        let mut read_buffer = vec![0; 1024];
        let bytes = stream_read.read(&mut read_buffer).await?;

        // FIXME: Error on invalid command
        let command = serde_json::from_slice::<Command>(&read_buffer[..bytes])?;

        debug!("Received command: {command:?}");

        cmd_tx.send_expect(command).await;
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

                let wl = ironbar.clients.borrow_mut().wayland();
                let outputs = wl.output_info_all();

                ironbar.reload_config();

                for output in outputs {
                    match crate::load_output_bars(ironbar, application, &output) {
                        Ok(mut bars) => ironbar.bars.borrow_mut().append(&mut bars),
                        Err(err) => error!("{err:?}"),
                    }
                }

                Response::Ok
            }
            Command::LoadCss { path } => {
                if path.exists() {
                    load_css(path, application.clone());
                    Response::Ok
                } else {
                    Response::error("File not found")
                }
            }
            Command::Var(cmd) => ironvar::handle_command(cmd),
            Command::Bar(cmd) => bar::handle_command(cmd, ironbar),
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
