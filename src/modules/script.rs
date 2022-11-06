use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::script::exec_command;
use color_eyre::{Help, Report, Result};
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;
use tokio::{select, spawn};
use tracing::error;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    Poll,
    Watch,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScriptModule {
    /// Path to script to execute.
    path: String,
    /// Script execution mode
    #[serde(default = "default_mode")]
    mode: Mode,
    /// Time in milliseconds between executions.
    #[serde(default = "default_interval")]
    interval: u64,
}

/// `Mode::Poll`
const fn default_mode() -> Mode {
    Mode::Poll
}

/// 5000ms
const fn default_interval() -> u64 {
    5000
}

impl Module<Label> for ScriptModule {
    type SendMessage = String;
    type ReceiveMessage = ();

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        _rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let interval = self.interval;
        let path = self.path.clone();

        match self.mode {
            Mode::Poll => spawn(async move {
                loop {
                    match exec_command(&path) {
                        Ok(stdout) => tx
                            .send(ModuleUpdateEvent::Update(stdout))
                            .await
                            .expect("Failed to send stdout"),
                        Err(err) => error!("{:?}", err),
                    }

                    sleep(tokio::time::Duration::from_millis(interval)).await;
                }
            }),
            Mode::Watch => spawn(async move {
                loop {
                    let mut handle = Command::new("sh")
                        .args(["-c", &path])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .stdin(Stdio::null())
                        .spawn()
                        .expect("Failed to spawn process");

                    let mut stdout_lines = BufReader::new(
                        handle
                            .stdout
                            .take()
                            .expect("Failed to take script handle stdout"),
                    )
                    .lines();

                    let mut stderr_lines = BufReader::new(
                        handle
                            .stderr
                            .take()
                            .expect("Failed to take script handle stderr"),
                    )
                    .lines();

                    loop {
                        select! {
                            _ = handle.wait() => break,
                            Ok(Some(line)) = stdout_lines.next_line() => {
                                tx.send(ModuleUpdateEvent::Update(line.to_string()))
                            .await
                            .expect("Failed to send stdout");
                            }
                            Ok(Some(line)) = stderr_lines.next_line() => {
                                error!("{:?}", Report::msg(line)
                                    .wrap_err("Watched script error:")
                                    .suggestion("Check the path to your script")
                                    .suggestion("Check the script for errors")
                                    .suggestion("If you expect the script to write to stderr, consider redirecting its output to /dev/null to suppress these messages")
                                )
                            }
                        }
                    }

                    while let Ok(Some(line)) = stdout_lines.next_line().await {
                        tx.send(ModuleUpdateEvent::Update(line.to_string()))
                            .await
                            .expect("Failed to send stdout");
                    }

                    sleep(tokio::time::Duration::from_millis(interval)).await;
                }
            }),
        };

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<Label>> {
        let label = Label::builder().use_markup(true).build();
        label.set_angle(info.bar_position.get_angle());

        {
            let label = label.clone();
            context.widget_rx.attach(None, move |s| {
                label.set_markup(s.as_str());
                Continue(true)
            });
        }

        Ok(ModuleWidget {
            widget: label,
            popup: None,
        })
    }
}
