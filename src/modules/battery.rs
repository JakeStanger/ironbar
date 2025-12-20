use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::upower;
use crate::clients::upower::BatteryState;
use crate::config::{CommonConfig, LayoutConfig, default};
use crate::gtk_helpers::IronbarLabelExt;
use crate::image::IconLabel;
use crate::modules::PopupButton;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::{module_impl, spawn};
use color_eyre::Result;
use futures_lite::stream::StreamExt;
use gtk::{Button, prelude::*};
use gtk::{Label, Orientation};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Write};
use tokio::sync::mpsc;
use zbus::zvariant::OwnedValue;

const DAY: i64 = 24 * 60 * 60;
const HOUR: i64 = 60 * 60;
const MINUTE: i64 = 60;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BatteryModule {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{percentage}%`
    format: String,

    /// The size to render the icon at, in pixels.
    ///
    /// **Default**: `24`
    icon_size: i32,

    /// Whether to show the icon.
    ///
    /// **Default**: `true`
    show_icon: bool,

    /// Whether to show the label.
    ///
    /// **Default**: `true`
    show_label: bool,

    // -- Common --
    /// See [layout options](module-level-options#layout)
    #[serde(flatten)]
    layout: LayoutConfig,

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
    ///       type = "battery"
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
    thresholds: HashMap<Box<str>, f64>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for BatteryModule {
    fn default() -> Self {
        Self {
            format: "{percentage}%".to_string(),
            icon_size: default::IconSize::Small as i32,
            layout: LayoutConfig::default(),
            show_icon: true,
            show_label: true,
            thresholds: HashMap::new(),
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UpowerProperties {
    percentage: f64,
    icon_name: String,
    state: BatteryState,
    time_to_full: i64,
    time_to_empty: i64,
}

impl Module<Button> for BatteryModule {
    type SendMessage = UpowerProperties;
    type ReceiveMessage = ();

    module_impl!("battery");

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

            let mut properties: UpowerProperties = display_proxy
                .get_all(display_proxy.interface_name.clone())
                .await?
                .try_into()?;

            tx.send_update(properties.clone()).await;

            while let Some(signal) = prop_changed_stream.next().await {
                let args = signal.args().expect("Invalid signal arguments");
                if args.interface_name != display_proxy.interface_name {
                    continue;
                }

                for (key, value) in args.changed_properties {
                    match key {
                        "Percentage" => {
                            properties.percentage = value.downcast::<f64>().unwrap_or_default();
                        }
                        "IconName" => {
                            properties.icon_name = value.downcast::<String>().unwrap_or_default();
                        }
                        "State" => {
                            properties.state =
                                value.downcast_ref::<BatteryState>().unwrap_or_default();
                        }
                        "TimeToFull" => {
                            properties.time_to_full = value.downcast::<i64>().unwrap_or_default();
                        }
                        "TimeToEmpty" => {
                            properties.time_to_empty = value.downcast::<i64>().unwrap_or_default();
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
        let icon = match self.show_icon {
            true => {
                let icon = IconLabel::new("", self.icon_size, &context.ironbar.image_provider());
                icon.add_css_class("icon");
                Some(icon)
            }
            false => None,
        };
        let label = match self.show_label {
            true => {
                let label = Label::builder()
                    .label(&self.format)
                    .use_markup(true)
                    .justify(self.layout.justify.into())
                    .build();
                label.add_css_class("label");
                Some(label)
            }
            false => None,
        };

        let container = gtk::Box::new(self.layout.orientation(info), 5);
        container.add_css_class("contents");

        let button = Button::new();
        button.add_css_class("button");

        if let Some(i) = &icon {
            container.append(&**i);
        }
        if let Some(l) = &label {
            container.append(l);
        }
        button.set_child(Some(&container));

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let rx = context.subscribe();
        rx.recv_glib(
            (&button, &self.format, &self.thresholds),
            move |(button, format, thresholds), properties| {
                let percentage = properties.percentage;

                if let Some(l) = &label {
                    let state = properties.state;
                    let is_charging =
                        state == BatteryState::Charging || state == BatteryState::PendingCharge;
                    let time_remaining = if is_charging {
                        seconds_to_string(properties.time_to_full)
                    } else {
                        seconds_to_string(properties.time_to_empty)
                    }
                    .unwrap_or_default();
                    let format = format
                        .replace("{percentage}", &percentage.round().to_string())
                        .replace("{time_remaining}", &time_remaining)
                        .replace("{state}", &state.to_string());

                    l.set_label_escaped(&format);
                }

                if let Some(i) = &icon {
                    i.set_label(Some(&format!("icon:{}", properties.icon_name)));
                }

                if let Some(threshold) = get_threshold(percentage, thresholds) {
                    button.add_css_class(threshold);

                    for class in thresholds.keys() {
                        if **class != *threshold {
                            button.remove_css_class(class);
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
        label.add_css_class("details");
        container.append(&label);

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

impl TryFrom<HashMap<String, OwnedValue>> for UpowerProperties {
    type Error = zbus::zvariant::Error;

    fn try_from(properties: HashMap<String, OwnedValue>) -> std::result::Result<Self, Self::Error> {
        Ok(UpowerProperties {
            percentage: properties["Percentage"].downcast_ref::<f64>()?,
            icon_name: properties["IconName"].downcast_ref::<&str>()?.to_string(),
            state: properties["State"].downcast_ref::<BatteryState>()?,
            time_to_full: properties["TimeToFull"].downcast_ref::<i64>()?,
            time_to_empty: properties["TimeToEmpty"].downcast_ref::<i64>()?,
        })
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
