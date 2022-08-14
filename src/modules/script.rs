use crate::modules::{Module, ModuleInfo};
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use std::process::Command;
use tokio::spawn;
use tokio::time::sleep;

#[derive(Debug, Deserialize, Clone)]
pub struct ScriptModule {
    path: String,
    #[serde(default = "default_interval")]
    interval: u64,
}

/// 5000ms
const fn default_interval() -> u64 {
    5000
}

impl Module<Label> for ScriptModule {
    fn into_widget(self, _info: &ModuleInfo) -> Label {
        let label = Label::builder().use_markup(true).build();

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            loop {
                let output = Command::new("sh").arg("-c").arg(&self.path).output();
                if let Ok(output) = output {
                    let stdout = String::from_utf8(output.stdout)
                        .map(|output| output.trim().to_string())
                        .expect("Script output not valid UTF-8");

                    tx.send(stdout).unwrap();
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

        label
    }
}
