mod parser;
mod renderer;
mod token;

use crate::clients::sysinfo::TokenType;
use crate::config::{CommonConfig, LayoutConfig, ModuleOrientation};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::modules::sysinfo::token::Part;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{clients, glib_recv, module_impl, send_async, spawn, try_send};
use color_eyre::Result;
use gtk::Label;
use gtk::prelude::*;
use serde::Deserialize;
use std::time::Duration;
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

    /// The orientation by which the labels are laid out.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br>
    /// **Default** : `horizontal`
    direction: Option<ModuleOrientation>,

    // -- common --
    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

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

    pub const fn disks(self) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => intervals.disks,
        }
    }

    pub const fn networks(self) -> u64 {
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RefreshType {
    Memory,
    Cpu,
    Temps,
    Disks,
    Network,
    System,
}

impl TokenType {
    fn is_affected_by(self, refresh_type: RefreshType) -> bool {
        match self {
            Self::CpuFrequency | Self::CpuPercent => refresh_type == RefreshType::Cpu,
            Self::MemoryFree
            | Self::MemoryAvailable
            | Self::MemoryTotal
            | Self::MemoryUsed
            | Self::MemoryPercent
            | Self::SwapFree
            | Self::SwapTotal
            | Self::SwapUsed
            | Self::SwapPercent => refresh_type == RefreshType::Memory,
            Self::TempC | Self::TempF => refresh_type == RefreshType::Temps,
            Self::DiskFree
            | Self::DiskTotal
            | Self::DiskUsed
            | Self::DiskPercent
            | Self::DiskRead
            | Self::DiskWrite => refresh_type == RefreshType::Disks,
            Self::NetDown | Self::NetUp => refresh_type == RefreshType::Network,
            Self::LoadAverage1 | Self::LoadAverage5 | Self::LoadAverage15 => {
                refresh_type == RefreshType::System
            }
            Self::Uptime => refresh_type == RefreshType::System,
        }
    }
}

impl Module<gtk::Box> for SysInfoModule {
    type SendMessage = (usize, String);
    type ReceiveMessage = ();

    module_impl!("sysinfo");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let interval = self.interval;

        let client = context.client::<clients::sysinfo::Client>();

        let format_tokens = self
            .format
            .iter()
            .map(|format| parser::parse_input(format.as_str()))
            .collect::<Result<Vec<_>>>()?;

        for (i, token_set) in format_tokens.iter().enumerate() {
            let rendered = Part::render_all(token_set, &client, interval);
            try_send!(context.tx, ModuleUpdateEvent::Update((i, rendered)));
        }

        let (refresh_tx, mut refresh_rx) = mpsc::channel(16);

        macro_rules! spawn_refresh {
            ($refresh_type:expr, $func:ident) => {{
                let tx = refresh_tx.clone();
                spawn(async move {
                    loop {
                        send_async!(tx, $refresh_type);
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
            while let Some(refresh) = refresh_rx.recv().await {
                match refresh {
                    RefreshType::Memory => client.refresh_memory(),
                    RefreshType::Cpu => client.refresh_cpu(),
                    RefreshType::Temps => client.refresh_temps(),
                    RefreshType::Disks => client.refresh_disks(),
                    RefreshType::Network => client.refresh_network(),
                    RefreshType::System => client.refresh_load_average(),
                };

                for (i, token_set) in format_tokens.iter().enumerate() {
                    let is_affected = token_set
                        .iter()
                        .filter_map(|part| {
                            if let Part::Token(token) = part {
                                Some(token)
                            } else {
                                None
                            }
                        })
                        .any(|t| t.token.is_affected_by(refresh));

                    if is_affected {
                        let rendered = Part::render_all(token_set, &client, interval);
                        send_async!(tx, ModuleUpdateEvent::Update((i, rendered)));
                    }
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let layout = match self.direction {
            Some(orientation) => orientation.into(),
            None => self.layout.orientation(info),
        };

        let container = gtk::Box::new(layout, 10);

        let mut labels = Vec::new();

        for _ in &self.format {
            let label = Label::builder()
                .use_markup(true)
                // .angle(self.layout.angle(info))
                .justify(self.layout.justify.into())
                .build();

            label.add_class("item");

            container.append(&label);
            labels.push(label);
        }

        glib_recv!(context.subscribe(), data => {
            let label = &labels[data.0];
            label.set_label_escaped(&data.1);
        });

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
