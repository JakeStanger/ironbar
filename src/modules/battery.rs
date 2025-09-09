use color_eyre::Result;
use futures_lite::stream::StreamExt;
use gtk::{Button, prelude::*};
use gtk::{Label, Orientation};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};
use zbus;
use zbus::fdo::PropertiesProxy;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::upower;
use crate::clients::upower::BatteryState;
use crate::config::{CommonConfig, LayoutConfig};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
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
pub struct BatteryModule {
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

            let device_interface_name =
                zbus::names::InterfaceName::from_static_str("org.freedesktop.UPower.Device")
                    .expect("failed to create zbus InterfaceName");

            let properties = display_proxy.get_all(device_interface_name.clone()).await?;

            let percentage = properties["Percentage"]
                .downcast_ref::<f64>()
                .expect("expected percentage: f64 in HashMap of all properties");
            let icon_name = properties["IconName"]
                .downcast_ref::<&str>()
                .expect("expected IconName: str in HashMap of all properties")
                .to_string();
            let state = u32_to_battery_state(
                properties["State"]
                    .downcast_ref::<u32>()
                    .expect("expected State: u32 in HashMap of all properties"),
            )
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
                if args.interface_name != device_interface_name {
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
                            properties.state =
                                u32_to_battery_state(changed_value.downcast::<u32>().unwrap_or(0))
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
        let icon = gtk::Image::new();
        icon.add_css_class("icon");

        let label = Label::builder()
            .label(&self.format)
            .use_markup(true)
            .justify(self.layout.justify.into())
            .build();

        label.add_css_class("label");

        let container = gtk::Box::new(self.layout.orientation(info), 5);
        container.add_css_class("contents");

        let button = Button::new();
        button.add_css_class("button");

        container.append(&icon);
        container.append(&label);
        button.set_child(Some(&container));

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let format = self.format.clone();
        let image_provider = context.ironbar.image_provider();

        context.subscribe().recv_glib((), move |(), properties| {
            let state = properties.state;
            let is_charging =
                state == BatteryState::Charging || state == BatteryState::PendingCharge;
            let time_remaining = if is_charging {
                seconds_to_string(properties.time_to_full)
            } else {
                seconds_to_string(properties.time_to_empty)
            };
            let format = format
                .replace("{percentage}", &properties.percentage.to_string())
                .replace("{time_remaining}", &time_remaining)
                .replace("{state}", battery_state_to_string(state));

            let mut icon_name = String::from("icon:");
            icon_name.push_str(&properties.icon_name);

            image_provider.load_into_image(&icon_name, self.icon_size, false, &icon);

            label.set_label_escaped(&format);
        });

        let rx = context.subscribe();
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
        label.add_css_class("upower-details");
        container.append(&label);

        context
            .subscribe()
            .recv_glib((&label), |(label), properties| {
                let state = properties.state;
                let format = match state {
                    BatteryState::Charging | BatteryState::PendingCharge => {
                        let ttf = properties.time_to_full;
                        if ttf > 0 {
                            format!("Full in {}", seconds_to_string(ttf))
                        } else {
                            String::new()
                        }
                    }
                    BatteryState::Discharging | BatteryState::PendingDischarge => {
                        let tte = properties.time_to_empty;
                        if tte > 0 {
                            format!("Empty in {}", seconds_to_string(tte))
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

fn seconds_to_string(seconds: i64) -> String {
    let mut time_string = String::new();
    let days = seconds / (DAY);
    if days > 0 {
        time_string += &format!("{days}d");
    }
    let hours = (seconds % DAY) / HOUR;
    if hours > 0 {
        time_string += &format!(" {hours}h");
    }
    let minutes = (seconds % HOUR) / MINUTE;
    if minutes > 0 {
        time_string += &format!(" {minutes}m");
    }
    time_string.trim_start().to_string()
}

const fn u32_to_battery_state(number: u32) -> Result<BatteryState, u32> {
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
        Err(number)
    }
}

fn battery_state_to_string(state: BatteryState) -> &'static str {
    match state {
        BatteryState::Unknown => "Unknown",
        BatteryState::Charging => "Charging",
        BatteryState::Discharging => "Discharging",
        BatteryState::Empty => "Empty",
        BatteryState::FullyCharged => "Fully charged",
        BatteryState::PendingCharge => "Pending charge",
        BatteryState::PendingDischarge => "Pending discharge",
    }
}
