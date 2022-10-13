use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::Label;
use regex::{Captures, Regex};
use serde::Deserialize;
use std::collections::HashMap;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;

#[derive(Debug, Deserialize, Clone)]
pub struct SysInfoModule {
    /// List of formatting strings.
    format: Vec<String>,
}

impl Module<gtk::Box> for SysInfoModule {
    type SendMessage = HashMap<String, String>;
    type ReceiveMessage = ();

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        _rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        spawn(async move {
            let mut sys = System::new_all();

            loop {
                sys.refresh_all();

                let mut format_info = HashMap::new();

                let actual_used_memory = sys.total_memory() - sys.available_memory();
                let memory_percent = actual_used_memory as f64 / sys.total_memory() as f64 * 100.0;

                let cpu_percent = sys.global_cpu_info().cpu_usage();

                // TODO: Add remaining format info

                format_info.insert(
                    String::from("memory-percent"),
                    format!("{:0>2.0}", memory_percent),
                );
                format_info.insert(
                    String::from("cpu-percent"),
                    format!("{:0>2.0}", cpu_percent),
                );

                tx.send(ModuleUpdateEvent::Update(format_info))
                    .await
                    .expect("Failed to send system info map");

                sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<gtk::Box>> {
        let re = Regex::new(r"\{([\w-]+)}")?;

        let container = gtk::Box::new(info.bar_position.get_orientation(), 10);

        let mut labels = Vec::new();

        for format in &self.format {
            let label = Label::builder().label(format).name("item").build();
            label.set_angle(info.bar_position.get_angle());
            container.add(&label);
            labels.push(label);
        }

        {
            let formats = self.format;
            context.widget_rx.attach(None, move |info| {
                for (format, label) in formats.iter().zip(labels.clone()) {
                    let format_compiled = re.replace(format, |caps: &Captures| {
                        info.get(&caps[1])
                            .unwrap_or(&caps[0].to_string())
                            .to_string()
                    });

                    label.set_text(format_compiled.as_ref());
                }

                Continue(true)
            });
        }

        Ok(ModuleWidget {
            widget: container,
            popup: None,
        })
    }
}
