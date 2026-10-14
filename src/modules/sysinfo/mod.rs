mod parser;
mod renderer;
mod token;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::sysinfo::RefreshType;
use crate::config::{CommonConfig, LayoutConfig, ModuleOrientation};
use crate::gtk_helpers::IronbarLabelExt;
use crate::modules::sysinfo::token::Part;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{clients, module_impl, spawn};
use color_eyre::Result;
use gtk::Label;
use gtk::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
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
    interval: Interval,

    /// The orientation by which the labels are laid out.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br>
    /// **Default** : `horizontal`
    direction: Option<ModuleOrientation>,

    // -- common --
    /// See [layout options](module-level-options#layout)
    #[serde(flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for SysInfoModule {
    fn default() -> Self {
        Self {
            format: vec![],
            interval: Interval::default(),
            direction: None,
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Deserialize, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Intervals {
    /// The number of seconds between refreshing memory data.
    ///
    /// **Default**: `5`
    memory: u64,

    /// The number of seconds between refreshing CPU data.
    ///
    /// **Default**: `5`
    cpu: u64,

    /// The number of seconds between refreshing temperature data.
    ///
    /// **Default**: `5`
    temps: u64,

    /// The number of seconds between refreshing disk data.
    ///
    /// **Default**: `5`
    disks: u64,

    /// The number of seconds between refreshing network data.
    ///
    /// **Default**: `5`
    networks: u64,

    /// The number of seconds between refreshing system data.
    ///
    /// **Default**: `5`
    system: u64,
}

impl Default for Intervals {
    fn default() -> Self {
        Self {
            memory: 5,
            cpu: 5,
            temps: 5,
            disks: 5,
            networks: 5,
            system: 5,
        }
    }
}

#[derive(Debug, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(untagged)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum Interval {
    All(u64),
    Individual(Intervals),
}

impl Default for Interval {
    fn default() -> Self {
        Self::All(5)
    }
}

impl Interval {
    const fn get(self, kind: RefreshType) -> u64 {
        match self {
            Self::All(n) => n,
            Self::Individual(intervals) => match kind {
                RefreshType::Memory => intervals.memory,
                RefreshType::Cpu => intervals.cpu,
                RefreshType::Temps => intervals.temps,
                RefreshType::Disks => intervals.disks,
                RefreshType::Network => intervals.networks,
                RefreshType::System => intervals.system,
            },
        }
    }
}

/// Returns whether any token in the format string takes its value from the given kind of data.
fn uses(token_set: &[Part], kind: RefreshType) -> bool {
    token_set
        .iter()
        .any(|part| matches!(part, Part::Token(token) if token.token.refresh_type() == kind))
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

        let intervals = RefreshType::ALL
            .into_iter()
            .filter(|&kind| format_tokens.iter().any(|token_set| uses(token_set, kind)))
            .map(|kind| (kind, interval.get(kind)))
            .collect();

        let mut refresh_rx = client.subscribe(intervals);

        for (i, token_set) in format_tokens.iter().enumerate() {
            let rendered = Part::render_all(token_set, &client);
            context.tx.send_update_spawn((i, rendered));
        }

        let tx = context.tx.clone();
        spawn(async move {
            while let Some(refreshed) = refresh_rx.recv().await {
                for (i, token_set) in format_tokens.iter().enumerate() {
                    if refreshed.iter().any(|&kind| uses(token_set, kind)) {
                        let rendered = Part::render_all(token_set, &client);
                        tx.send_update((i, rendered)).await;
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
                .justify(self.layout.justify.into())
                .build();

            label.add_css_class("item");

            container.append(&label);
            labels.push(label);
        }

        context.subscribe().recv_glib((), move |(), data| {
            let label = &labels[data.0];
            label.set_label_escaped(&data.1);
        });

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
