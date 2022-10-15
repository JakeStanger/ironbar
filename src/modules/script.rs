use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use color_eyre::{eyre::Report, eyre::Result, eyre::WrapErr, Section};
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use std::process::Command;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;
use tracing::{error, instrument};

#[derive(Debug, Deserialize, Clone)]
pub struct ScriptModule {
    /// Path to script to execute.
    path: String,
    /// Time in milliseconds between executions.
    #[serde(default = "default_interval")]
    interval: u64,
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
        spawn(async move {
            loop {
                match run_script(&path) {
                    Ok(stdout) => tx
                        .send(ModuleUpdateEvent::Update(stdout))
                        .await
                        .expect("Failed to send stdout"),
                    Err(err) => error!("{:?}", err),
                }

                sleep(tokio::time::Duration::from_millis(interval)).await;
            }
        });

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
                label.set_label(s.as_str());
                Continue(true)
            });
        }

        Ok(ModuleWidget {
            widget: label,
            popup: None,
        })
    }
}

#[instrument]
fn run_script(path: &str) -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(path)
        .output()
        .wrap_err("Failed to get script output")?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map(|output| output.trim().to_string())
            .wrap_err("Script stdout not valid UTF-8")?;

        Ok(stdout)
    } else {
        let stderr = String::from_utf8(output.stderr)
            .map(|output| output.trim().to_string())
            .wrap_err("Script stderr not valid UTF-8")?;

        Err(Report::msg(stderr)
            .wrap_err("Script returned non-zero error code")
            .suggestion("Check the path to your script")
            .suggestion("Check the script for errors"))
    }
}
