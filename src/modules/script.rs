use crate::modules::{Module, ModuleInfo};
use color_eyre::{eyre::Report, eyre::Result, eyre::WrapErr, Section};
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use std::process::Command;
use tokio::spawn;
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
    fn into_widget(self, _info: &ModuleInfo) -> Result<Label> {
        let label = Label::builder().use_markup(true).build();

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            loop {
                match self.run_script() {
                    Ok(stdout) => tx.send(stdout).expect("Failed to send stdout"),
                    Err(err) => error!("{:?}", err),
                }

                sleep(tokio::time::Duration::from_millis(self.interval)).await;
            }
        });

        {
            let label = label.clone();
            rx.attach(None, move |s| {
                label.set_label(s.as_str());
                Continue(true)
            });
        }

        Ok(label)
    }
}

impl ScriptModule {
    #[instrument]
    fn run_script(&self) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&self.path)
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
}
