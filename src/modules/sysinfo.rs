use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::{CommonConfig, ModuleOrientation};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::Label;
use regex::{Captures, Regex};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use sysinfo::{ComponentExt, CpuExt, DiskExt, NetworkExt, RefreshKind, System, SystemExt};
use tokio::sync::mpsc;
use tokio::time::sleep;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SysInfoModule {
    /// List of strings including formatting tokens.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Required**
    format: Vec<String>,

    /// Number of seconds between refresh.
    ///
    /// This can be set as a global interval,
    /// or passed as an object to customize the interval per-system.
    ///
    /// **Default**: `5`
    #[serde(default = "Interval::default")]
    interval: Interval,

    /// The orientation of text for the labels.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br>
    /// **Default** : `horizontal`
    #[serde(default)]
    orientation: ModuleOrientation,

    /// The orientation by which the labels are laid out.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br>
    /// **Default** : `horizontal`
    direction: Option<ModuleOrientation>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Copy, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Intervals {
    /// The number of seconds between refreshing memory data.
    ///
    /// **Default**: `5`
    #[serde(default = "default_interval")]
    memory: u64,

    /// The number of seconds between refreshing CPU data.
    ///
    /// **Default**: `5`
    #[serde(default = "default_interval")]
    cpu: u64,

    /// The number of seconds between refreshing temperature data.
    ///
    /// **Default**: `5`
    #[serde(default = "default_interval")]
    temps: u64,

    /// The number of seconds between refreshing disk data.
    ///
    /// **Default**: `5`
    #[serde(default = "default_interval")]
    disks: u64,

    /// The number of seconds between refreshing network data.
    ///
    /// **Default**: `5`
    #[serde(default = "default_interval")]
    networks: u64,

    /// The number of seconds between refreshing system data.
    ///
    /// **Default**: `5`
    #[serde(default = "default_interval")]
    system: u64,
}

#[derive(Debug, Deserialize, Copy, Clone)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

    module_impl!("sysinfo");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
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

        macro_rules! spawn_refresh {
            ($refresh_type:expr, $func:ident) => {{
                let tx = refresh_tx.clone();
                spawn(async move {
                    loop {
                        tx.send_expect($refresh_type).await;
                        sleep(Duration::from_secs(interval.$func())).await;
                    }
                });
            }};
        }

        spawn_refresh!(RefreshType::Memory, memory);
        spawn_refresh!(RefreshType::Cpu, cpu);
        spawn_refresh!(RefreshType::Temps, temps);
        spawn_refresh!(RefreshType::Disks, disks);
        spawn_refresh!(RefreshType::Network, networks);
        spawn_refresh!(RefreshType::System, system);

        let tx = context.tx.clone();
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

                tx.send_update(format_info.clone()).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let re = Regex::new(r"\{([^}]+)}")?;

        let layout = match self.direction {
            Some(orientation) => orientation,
            None => self.orientation,
        };

        let container = gtk::Box::new(layout.into(), 10);

        let mut labels = Vec::new();

        for format in &self.format {
            let label = Label::builder().label(format).use_markup(true).build();

            label.add_class("item");
            label.set_angle(self.orientation.to_angle());

            container.add(&label);
            labels.push(label);
        }

        {
            let formats = self.format;
            context.subscribe().recv_glib(move |info| {
                for (format, label) in formats.iter().zip(labels.clone()) {
                    let format_compiled = re.replace_all(format, |caps: &Captures| {
                        info.get(&caps[1])
                            .unwrap_or(&caps[0].to_string())
                            .to_string()
                    });

                    label.set_label_escaped(format_compiled.as_ref());
                }
            });
        }

        Ok(ModuleParts {
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
        String::from("memory_free"),
        (bytes_to_gigabytes(available_memory)).to_string(),
    );
    format_info.insert(
        String::from("memory_used"),
        (bytes_to_gigabytes(actual_used_memory)).to_string(),
    );
    format_info.insert(
        String::from("memory_total"),
        (bytes_to_gigabytes(total_memory)).to_string(),
    );
    format_info.insert(
        String::from("memory_percent"),
        format!("{memory_percent:0>2.0}"),
    );

    let used_swap = sys.used_swap();
    let total_swap = sys.total_swap();

    format_info.insert(
        String::from("swap_free"),
        (bytes_to_gigabytes(sys.free_swap())).to_string(),
    );
    format_info.insert(
        String::from("swap_used"),
        (bytes_to_gigabytes(used_swap)).to_string(),
    );
    format_info.insert(
        String::from("swap_total"),
        (bytes_to_gigabytes(total_swap)).to_string(),
    );
    format_info.insert(
        String::from("swap_percent"),
        format!("{:0>2.0}", used_swap as f64 / total_swap as f64 * 100.0),
    );
}

fn refresh_cpu_tokens(format_info: &mut HashMap<String, String>, sys: &mut System) {
    sys.refresh_cpu();

    let cpu_info = sys.global_cpu_info();
    let cpu_percent = cpu_info.cpu_usage();

    format_info.insert(String::from("cpu_percent"), format!("{cpu_percent:0>2.0}"));
}

fn refresh_temp_tokens(format_info: &mut HashMap<String, String>, sys: &mut System) {
    sys.refresh_components();

    let components = sys.components();
    for component in components {
        let key = component.label().replace(' ', "-");
        let temp = component.temperature();

        format_info.insert(format!("temp_c:{key}"), format!("{temp:.0}"));
        format_info.insert(format!("temp_f:{key}"), format!("{:.0}", c_to_f(temp)));
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
                format!("disk_free:{key}"),
                bytes_to_gigabytes(available).to_string(),
            );

            format_info.insert(
                format!("disk_used:{key}"),
                bytes_to_gigabytes(used).to_string(),
            );

            format_info.insert(
                format!("disk_total:{key}"),
                bytes_to_gigabytes(total).to_string(),
            );

            format_info.insert(
                format!("disk_percent:{key}"),
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
            format!("net_down:{iface}"),
            format!("{:0>2.0}", bytes_to_megabits(network.received()) / interval),
        );

        format_info.insert(
            format!("net_up:{iface}"),
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
    format_info.insert(
        String::from("load_average:1"),
        format!("{:.2}", load_average.one),
    );

    format_info.insert(
        String::from("load_average:5"),
        format!("{:.2}", load_average.five),
    );

    format_info.insert(
        String::from("load_average:15"),
        format!("{:.2}", load_average.fifteen),
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
