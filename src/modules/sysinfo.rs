use crate::modules::{Module, ModuleInfo};
use gtk::prelude::*;
use gtk::{Label, Orientation};
use regex::{Captures, Regex};
use serde::Deserialize;
use std::collections::HashMap;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::spawn;
use tokio::time::sleep;

#[derive(Debug, Deserialize, Clone)]
pub struct SysInfoModule {
    format: Vec<String>,
}

impl Module<gtk::Box> for SysInfoModule {
    fn into_widget(self, _info: &ModuleInfo) -> gtk::Box {
        let re = Regex::new(r"\{([\w-]+)}").unwrap();

        let container = gtk::Box::new(Orientation::Horizontal, 10);

        let mut labels = Vec::new();

        for format in &self.format {
            let label = Label::builder().label(format).name("item").build();
            container.add(&label);
            labels.push(label);
        }

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            let mut sys = System::new_all();

            loop {
                sys.refresh_all();

                let mut format_info = HashMap::new();

                let actual_used_memory = sys.total_memory() - sys.available_memory();
                let memory_percent = actual_used_memory as f64 / sys.total_memory() as f64 * 100.0;

                let cpu_percent = sys.global_cpu_info().cpu_usage();

                // TODO: Add remaining format info

                format_info.insert("memory-percent", format!("{:0>2.0}", memory_percent));
                format_info.insert("cpu-percent", format!("{:0>2.0}", cpu_percent));

                tx.send(format_info).unwrap();

                sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        {
            let formats = self.format;
            rx.attach(None, move |info| {
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

        container
    }
}
