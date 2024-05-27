use color_eyre::Result;
use futures_lite::stream::StreamExt;
use gtk::{prelude::*, Button};
use gtk::{Label, Orientation};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};
use upower_dbus::BatteryState;
use zbus;
use zbus::fdo::PropertiesProxy;

use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::ImageProvider;
use crate::modules::PopupButton;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::{glib_recv, module_impl, send_async, spawn, try_send};

const DAY: i64 = 24 * 60 * 60;
const HOUR: i64 = 60 * 60;
const MINUTE: i64 = 60;

#[derive(Debug, Deserialize, Clone)]
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

impl Module<gtk::Button> for UpowerModule {
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

        let display_proxy = context.client::<PropertiesProxy>();

        spawn(async move {
            let mut prop_changed_stream = display_proxy.receive_properties_changed().await?;

            let device_interface_name =
                zbus::names::InterfaceName::from_static_str("org.freedesktop.UPower.Device")
                    .expect("failed to create zbus InterfaceName");

            let properties = display_proxy.get_all(device_interface_name.clone()).await?;

            let percentage = *properties["Percentage"]
                .downcast_ref::<f64>()
                .expect("expected percentage: f64 in HashMap of all properties");
            let icon_name = properties["IconName"]
                .downcast_ref::<str>()
                .expect("expected IconName: str in HashMap of all properties")
                .to_string();
            let state = u32_to_battery_state(
                *properties["State"]
                    .downcast_ref::<u32>()
                    .expect("expected State: u32 in HashMap of all properties"),
            )
            .unwrap_or(BatteryState::Unknown);
            let time_to_full = *properties["TimeToFull"]
                .downcast_ref::<i64>()
                .expect("expected TimeToFull: i64 in HashMap of all properties");
            let time_to_empty = *properties["TimeToEmpty"]
                .downcast_ref::<i64>()
                .expect("expected TimeToEmpty: i64 in HashMap of all properties");
            let mut properties = UpowerProperties {
                percentage,
                icon_name: icon_name.clone(),
                state,
                time_to_full,
                time_to_empty,
            };

            send_async!(tx, ModuleUpdateEvent::Update(properties.clone()));

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
                                .downcast_ref::<str>()
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

                send_async!(tx, ModuleUpdateEvent::Update(properties.clone()));
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
        let icon_theme = info.icon_theme.clone();
        let icon = gtk::Image::new();
        icon.add_class("icon");

        let label = Label::builder()
            .label(&self.format)
            .use_markup(true)
            .build();
        label.add_class("label");

        let container = gtk::Box::new(Orientation::Horizontal, 5);
        container.add_class("contents");

        let button = Button::new();
        button.add_class("button");

        container.add(&icon);
        container.add(&label);
        button.add(&container);

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            try_send!(tx, ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let format = self.format.clone();

        let rx = context.subscribe();
        glib_recv!(rx, properties => {
            let state = properties.state;
            let is_charging = state == BatteryState::Charging || state == BatteryState::PendingCharge;
            let time_remaining = if is_charging {
                seconds_to_string(properties.time_to_full)
            }
            else {
                seconds_to_string(properties.time_to_empty)
            };
            let format = format.replace("{percentage}", &properties.percentage.to_string())
                .replace("{time_remaining}", &time_remaining)
                .replace("{state}", battery_state_to_string(state));

            let mut icon_name = String::from("icon:");
            icon_name.push_str(&properties.icon_name);

            ImageProvider::parse(&icon_name, &icon_theme, false, self.icon_size)
                    .map(|provider| provider.load_into_image(icon.clone()));

            label.set_markup(format.as_ref());
        });

        let rx = context.subscribe();
        let popup = self
            .into_popup(context.controller_tx.clone(), rx, context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        _tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .build();

        let label = Label::new(None);
        label.add_class("upower-details");
        container.add(&label);

        glib_recv!(rx, properties => {
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

            label.set_markup(&format);
        });

        container.show_all();

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
