use color_eyre::Result;
use futures_lite::stream::StreamExt;
use gtk::{Button, prelude::*};
use gtk::{Label, Orientation};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Write};
use tokio::sync::mpsc;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::upower;
use crate::clients::upower::BatteryState;
use crate::config::{CommonConfig, LayoutConfig};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::image::IconLabel;
use crate::modules::PopupButton;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::{module_impl, spawn};

const DAY: i64 = 24 * 60 * 60;
const HOUR: i64 = 60 * 60;
const MINUTE: i64 = 60;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UpowerModule {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{percentage}%`
    #[serde(default = "default_format")]
    format: String,

    /// The size to render the icon at, in pixels.
    ///
    /// **Default**: `24`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    /// A map of threshold names to apply as classes,
    /// against the battery percentage at which to apply them.
    ///
    /// Thresholds work by applying the nearest value
    /// above the current percentage, if present.
    ///
    /// For example, using the below config:
    /// ```corn
    /// {
    ///   end = [
    ///     {
    ///       type = "upower"
    ///       format = "{percentage}%"
    ///       thresholds.warning = 20
    ///       thresholds.critical = 5
    ///     }
    ///   ]
    /// }
    /// ```
    /// At battery levels below 20%,
    /// the `.warning` class will be applied to the top-level widget.
    /// Below 5%, `.critical` will be applied instead.
    /// Above 20%, no class applies.
    ///
    /// **Default**: `{}`
    #[serde(default)]
    thresholds: HashMap<Box<str>, f64>,

    // -- Common --
    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_format() -> String {
    String::from("{percentage}%")
}

const fn default_icon_size() -> i32 {
    24
}

#[derive(Clone, Debug)]
pub struct UpowerProperties {
    percentage: f64,
    icon_name: String,
    state: BatteryState,
    time_to_full: i64,
    time_to_empty: i64,
}

impl Module<Button> for UpowerModule {
    type SendMessage = UpowerProperties;
    type ReceiveMessage = ();

    module_impl!("upower");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();

        let display_proxy = context.try_client::<upower::Client>()?;

        spawn(async move {
            let mut prop_changed_stream = display_proxy.receive_properties_changed().await?;

            let properties = display_proxy
                .get_all(display_proxy.interface_name.clone())
                .await?;

            let percentage = properties["Percentage"]
                .downcast_ref::<f64>()
                .expect("expected percentage: f64 in HashMap of all properties");

            let icon_name = properties["IconName"]
                .downcast_ref::<&str>()
                .expect("expected IconName: str in HashMap of all properties")
                .to_string();

            let state = properties["State"]
                .downcast_ref::<u32>()
                .expect("expected State: u32 in HashMap of all properties")
                .try_into()
                .unwrap_or(BatteryState::Unknown);

            let time_to_full = properties["TimeToFull"]
                .downcast_ref::<i64>()
                .expect("expected TimeToFull: i64 in HashMap of all properties");

            let time_to_empty = properties["TimeToEmpty"]
                .downcast_ref::<i64>()
                .expect("expected TimeToEmpty: i64 in HashMap of all properties");

            let mut properties = UpowerProperties {
                percentage,
                icon_name: icon_name.clone(),
                state,
                time_to_full,
                time_to_empty,
            };

            tx.send_update(properties.clone()).await;

            while let Some(signal) = prop_changed_stream.next().await {
                let args = signal.args().expect("Invalid signal arguments");
                if args.interface_name != display_proxy.interface_name {
                    continue;
                }

                for (name, changed_value) in args.changed_properties {
                    match name {
                        "Percentage" => {
                            properties.percentage = changed_value
                                .downcast::<f64>()
                                .expect("expected Percentage to be f64");
                        }
                        "IconName" => {
                            properties.icon_name = changed_value
                                .downcast_ref::<&str>()
                                .expect("expected IconName to be str")
                                .to_string();
                        }
                        "State" => {
                            properties.state = changed_value
                                .downcast::<u32>()
                                .unwrap_or(0)
                                .try_into()
                                .expect("expected State to be BatteryState");
                        }
                        "TimeToFull" => {
                            properties.time_to_full = changed_value
                                .downcast::<i64>()
                                .expect("expected TimeToFull to be i64");
                        }
                        "TimeToEmpty" => {
                            properties.time_to_empty = changed_value
                                .downcast::<i64>()
                                .expect("expected TimeToEmpty to be i64");
                        }
                        _ => {}
                    }
                }

                tx.send_update(properties.clone()).await;
            }

            Result::<()>::Ok(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<Button>> {
        let icon = IconLabel::new("", self.icon_size, &context.ironbar.image_provider());
        icon.add_class("icon");

        let label = Label::builder()
            .label(&self.format)
            .use_markup(true)
            .angle(self.layout.angle(info))
            .justify(self.layout.justify.into())
            .build();

        label.add_class("label");

        let container = gtk::Box::new(self.layout.orientation(info), 5);
        container.add_class("contents");

        let button = Button::new();
        button.add_class("button");

        container.add(&*icon);
        container.add(&label);
        button.add(&container);

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let rx = context.subscribe();
        rx.recv_glib(
            (&button, &self.format, &self.thresholds),
            move |(button, format, thresholds), properties| {
                let state = properties.state;

                let is_charging =
                    state == BatteryState::Charging || state == BatteryState::PendingCharge;

                let time_remaining = if is_charging {
                    seconds_to_string(properties.time_to_full)
                } else {
                    seconds_to_string(properties.time_to_empty)
                }
                .unwrap_or_default();

                let percentage = properties.percentage;
                let format = format
                    .replace("{percentage}", &percentage.round().to_string())
                    .replace("{time_remaining}", &time_remaining)
                    .replace("{state}", &state.to_string());

                label.set_label_escaped(&format);
                icon.set_label(Some(&format!("icon:{}", properties.icon_name)));

                if let Some(threshold) = get_threshold(percentage, thresholds) {
                    button.add_class(threshold);

                    for class in thresholds.keys() {
                        if **class != *threshold {
                            button.remove_class(class);
                        }
                    }
                }
            },
        );

        let popup = self
            .into_popup(context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .build();

        let label = Label::builder().use_markup(true).build();
        label.add_class("upower-details");
        container.add(&label);

        context.subscribe().recv_glib((), move |(), properties| {
            let state = properties.state;
            let format = match state {
                BatteryState::Charging | BatteryState::PendingCharge => {
                    let ttf = properties.time_to_full;
                    if ttf > 0 {
                        format!("Full in {}", seconds_to_string(ttf).unwrap_or_default())
                    } else {
                        String::new()
                    }
                }
                BatteryState::Discharging | BatteryState::PendingDischarge => {
                    let tte = properties.time_to_empty;
                    if tte > 0 {
                        format!("Empty in {}", seconds_to_string(tte).unwrap_or_default())
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            };

            label.set_label_escaped(&format);
        });

        container.show_all();

        Some(container)
    }
}

fn get_threshold(percent: f64, thresholds: &HashMap<Box<str>, f64>) -> Option<&str> {
    let mut candidates = thresholds
        .iter()
        .filter(|&(_, v)| *v >= percent)
        .collect::<Vec<_>>();

    candidates.sort_by(|&(_, v1), &(_, v2)| v2.partial_cmp(v1).unwrap_or(Ordering::Equal));

    if let Some((key, _)) = candidates.first() {
        Some(key)
    } else {
        None
    }
}

fn seconds_to_string(seconds: i64) -> Result<String> {
    let mut time_string = String::new();
    let days = seconds / (DAY);
    if days > 0 {
        write!(time_string, "{days}d")?;
    }
    let hours = (seconds % DAY) / HOUR;
    if hours > 0 {
        write!(time_string, " {hours}h")?;
    }
    let minutes = (seconds % HOUR) / MINUTE;
    if minutes > 0 {
        write!(time_string, " {minutes}m")?;
    }

    Ok(time_string.trim_start().to_string())
}

impl TryFrom<u32> for BatteryState {
    type Error = ();

    fn try_from(number: u32) -> std::result::Result<Self, Self::Error> {
        if number == (BatteryState::Unknown as u32) {
            Ok(BatteryState::Unknown)
        } else if number == (BatteryState::Charging as u32) {
            Ok(BatteryState::Charging)
        } else if number == (BatteryState::Discharging as u32) {
            Ok(BatteryState::Discharging)
        } else if number == (BatteryState::Empty as u32) {
            Ok(BatteryState::Empty)
        } else if number == (BatteryState::FullyCharged as u32) {
            Ok(BatteryState::FullyCharged)
        } else if number == (BatteryState::PendingCharge as u32) {
            Ok(BatteryState::PendingCharge)
        } else if number == (BatteryState::PendingDischarge as u32) {
            Ok(BatteryState::PendingDischarge)
        } else {
            Err(())
        }
    }
}

impl Display for BatteryState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BatteryState::Unknown => "Unknown",
                BatteryState::Charging => "Charging",
                BatteryState::Discharging => "Discharging",
                BatteryState::Empty => "Empty",
                BatteryState::FullyCharged => "Fully charged",
                BatteryState::PendingCharge => "Pending charge",
                BatteryState::PendingDischarge => "Pending discharge",
            }
        )
    }
}
