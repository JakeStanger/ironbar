use crate::config::CommonConfig;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::Label;
use regex::{Captures, Regex};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use sysinfo::{ComponentExt, CpuExt, DiskExt, NetworkExt, RefreshKind, System, SystemExt};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;

#[derive(Debug, Deserialize, Clone)]
pub struct SysInfoModule {
    /// List of formatting strings.
    format: Vec<String>,
    /// Number of seconds between refresh
    #[serde(default = "Interval::default")]
    interval: Interval,

    #[serde(flatten)]
    pub common: CommonConfig,
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub struct Intervals {
    #[serde(default = "default_interval")]
    memory: u64,
    #[serde(default = "default_interval")]
    cpu: u64,
    #[serde(default = "default_interval")]
    temps: u64,
    #[serde(default = "default_interval")]
    disks: u64,
    #[serde(default = "default_interval")]
    networks: u64,
    #[serde(default = "default_interval")]
    system: u64,
}

#[derive(Debug, Deserialize, Copy, Clone)]
#[serde(untagged)]
pub enum Interval {
    All(u64),
    Individual(Intervals),
}

impl Default for Interval {
    fn default() -> Self {
        Self::All(default_interval())
    }
}

impl Interval {
    const fn memory(self) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => intervals.memory,
        }
    }

    const fn cpu(self) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => intervals.cpu,
        }
    }

    const fn temps(self) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => intervals.temps,
        }
    }

    const fn disks(self) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => intervals.disks,
        }
    }

    const fn networks(self) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => intervals.networks,
        }
    }

    const fn system(self) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => intervals.system,
        }
    }
}

const fn default_interval() -> u64 {
    5
}

#[derive(Debug)]
enum RefreshType {
    Memory,
    Cpu,
    Temps,
    Disks,
    Network,
    System,
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
        let interval = self.interval;

        let refresh_kind = RefreshKind::everything()
            .without_processes()
            .without_users_list();

        let mut sys = System::new_with_specifics(refresh_kind);
        sys.refresh_components_list();
        sys.refresh_disks_list();
        sys.refresh_networks_list();

        let (refresh_tx, mut refresh_rx) = mpsc::channel(16);

        // memory refresh
        {
            let tx = refresh_tx.clone();
            spawn(async move {
                loop {
                    tx.send(RefreshType::Memory)
                        .await
                        .expect("Failed to send memory refresh");
                    sleep(Duration::from_secs(interval.memory())).await;
                }
            });
        }

        // cpu refresh
        {
            let tx = refresh_tx.clone();
            spawn(async move {
                loop {
                    tx.send(RefreshType::Cpu)
                        .await
                        .expect("Failed to send cpu refresh");
                    sleep(Duration::from_secs(interval.cpu())).await;
                }
            });
        }

        // temp refresh
        {
            let tx = refresh_tx.clone();
            spawn(async move {
                loop {
                    tx.send(RefreshType::Temps)
                        .await
                        .expect("Failed to send temperature refresh");
                    sleep(Duration::from_secs(interval.temps())).await;
                }
            });
        }

        // disk refresh
        {
            let tx = refresh_tx.clone();
            spawn(async move {
                loop {
                    tx.send(RefreshType::Disks)
                        .await
                        .expect("Failed to send disk refresh");
                    sleep(Duration::from_secs(interval.disks())).await;
                }
            });
        }

        // network refresh
        {
            let tx = refresh_tx.clone();
            spawn(async move {
                loop {
                    tx.send(RefreshType::Network)
                        .await
                        .expect("Failed to send network refresh");
                    sleep(Duration::from_secs(interval.networks())).await;
                }
            });
        }

        // system refresh
        {
            let tx = refresh_tx;
            spawn(async move {
                loop {
                    tx.send(RefreshType::System)
                        .await
                        .expect("Failed to send system refresh");
                    sleep(Duration::from_secs(interval.system())).await;
                }
            });
        }

        spawn(async move {
            let mut format_info = HashMap::new();

            while let Some(refresh) = refresh_rx.recv().await {
                match refresh {
                    RefreshType::Memory => refresh_memory_tokens(&mut format_info, &mut sys),
                    RefreshType::Cpu => refresh_cpu_tokens(&mut format_info, &mut sys),
                    RefreshType::Temps => refresh_temp_tokens(&mut format_info, &mut sys),
                    RefreshType::Disks => refresh_disk_tokens(&mut format_info, &mut sys),
                    RefreshType::Network => {
                        refresh_network_tokens(&mut format_info, &mut sys, interval.networks());
                    }
                    RefreshType::System => refresh_system_tokens(&mut format_info, &sys),
                };

                tx.send(ModuleUpdateEvent::Update(format_info.clone()))
                    .await
                    .expect("Failed to send system info map");
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<gtk::Box>> {
        let re = Regex::new(r"\{([^}]+)}")?;

        let container = gtk::Box::new(info.bar_position.get_orientation(), 10);

        let mut labels = Vec::new();

        for format in &self.format {
            let label = Label::builder()
                .label(format)
                .use_markup(true)
                .name("item")
                .build();
            label.set_angle(info.bar_position.get_angle());
            container.add(&label);
            labels.push(label);
        }

        {
            let formats = self.format;
            context.widget_rx.attach(None, move |info| {
                for (format, label) in formats.iter().zip(labels.clone()) {
                    let format_compiled = re.replace_all(format, |caps: &Captures| {
                        info.get(&caps[1])
                            .unwrap_or(&caps[0].to_string())
                            .to_string()
                    });

                    label.set_markup(format_compiled.as_ref());
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

fn refresh_memory_tokens(format_info: &mut HashMap<String, String>, sys: &mut System) {
    sys.refresh_memory();

    let total_memory = sys.total_memory();
    let available_memory = sys.available_memory();

    let actual_used_memory = total_memory - available_memory;
    let memory_percent = actual_used_memory as f64 / total_memory as f64 * 100.0;

    format_info.insert(
        String::from("memory-free"),
        (bytes_to_gigabytes(available_memory)).to_string(),
    );
    format_info.insert(
        String::from("memory-used"),
        (bytes_to_gigabytes(actual_used_memory)).to_string(),
    );
    format_info.insert(
        String::from("memory-total"),
        (bytes_to_gigabytes(total_memory)).to_string(),
    );
    format_info.insert(
        String::from("memory-percent"),
        format!("{:0>2.0}", memory_percent),
    );

    let used_swap = sys.used_swap();
    let total_swap = sys.total_swap();

    format_info.insert(
        String::from("swap-free"),
        (bytes_to_gigabytes(sys.free_swap())).to_string(),
    );
    format_info.insert(
        String::from("swap-used"),
        (bytes_to_gigabytes(used_swap)).to_string(),
    );
    format_info.insert(
        String::from("swap-total"),
        (bytes_to_gigabytes(total_swap)).to_string(),
    );
    format_info.insert(
        String::from("swap-percent"),
        format!("{:0>2.0}", used_swap as f64 / total_swap as f64 * 100.0),
    );
}

fn refresh_cpu_tokens(format_info: &mut HashMap<String, String>, sys: &mut System) {
    sys.refresh_cpu();

    let cpu_info = sys.global_cpu_info();
    let cpu_percent = cpu_info.cpu_usage();

    format_info.insert(
        String::from("cpu-percent"),
        format!("{:0>2.0}", cpu_percent),
    );
}

fn refresh_temp_tokens(format_info: &mut HashMap<String, String>, sys: &mut System) {
    sys.refresh_components();

    let components = sys.components();
    for component in components {
        let key = component.label().replace(' ', "-");
        let temp = component.temperature();

        format_info.insert(format!("temp-c:{key}"), format!("{temp:.0}"));
        format_info.insert(format!("temp-f:{key}"), format!("{:.0}", c_to_f(temp)));
    }
}

fn refresh_disk_tokens(format_info: &mut HashMap<String, String>, sys: &mut System) {
    sys.refresh_disks();

    for disk in sys.disks() {
        // replace braces to avoid conflict with regex
        let key = disk
            .mount_point()
            .to_str()
            .map(|s| s.replace(['{', '}'], ""));

        if let Some(key) = key {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total - available;

            format_info.insert(
                format!("disk-free:{key}"),
                bytes_to_gigabytes(available).to_string(),
            );

            format_info.insert(
                format!("disk-used:{key}"),
                bytes_to_gigabytes(used).to_string(),
            );

            format_info.insert(
                format!("disk-total:{key}"),
                bytes_to_gigabytes(total).to_string(),
            );

            format_info.insert(
                format!("disk-percent:{key}"),
                format!("{:0>2.0}", used as f64 / total as f64 * 100.0),
            );
        }
    }
}

fn refresh_network_tokens(
    format_info: &mut HashMap<String, String>,
    sys: &mut System,
    interval: u64,
) {
    sys.refresh_networks();

    for (iface, network) in sys.networks() {
        format_info.insert(
            format!("net-down:{iface}"),
            format!("{:0>2.0}", bytes_to_megabits(network.received()) / interval),
        );

        format_info.insert(
            format!("net-up:{iface}"),
            format!(
                "{:0>2.0}",
                bytes_to_megabits(network.transmitted()) / interval
            ),
        );
    }
}

fn refresh_system_tokens(format_info: &mut HashMap<String, String>, sys: &System) {
    // no refresh required for these tokens

    let load_average = sys.load_average();
    format_info.insert(String::from("load-average:1"), load_average.one.to_string());

    format_info.insert(
        String::from("load-average:5"),
        load_average.five.to_string(),
    );

    format_info.insert(
        String::from("load-average:15"),
        load_average.fifteen.to_string(),
    );

    let uptime = Duration::from_secs(sys.uptime()).as_secs();
    let hours = uptime / 3600;
    format_info.insert(
        String::from("uptime"),
        format!("{:0>2}:{:0>2}", hours, (uptime % 3600) / 60),
    );
}

/// Converts celsius to fahrenheit.
fn c_to_f(c: f32) -> f32 {
    c * 9.0 / 5.0 + 32.0
}

const fn bytes_to_gigabytes(b: u64) -> u64 {
    const BYTES_IN_GIGABYTE: u64 = 1_000_000_000;
    b / BYTES_IN_GIGABYTE
}

const fn bytes_to_megabits(b: u64) -> u64 {
    const BYTES_IN_MEGABIT: u64 = 125_000;
    b / BYTES_IN_MEGABIT
}
