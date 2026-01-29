use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::upower;
use crate::clients::upower::BatteryState;
use crate::config::{CommonConfig, LayoutConfig, Profiles, State, default};
use crate::gtk_helpers::IronbarLabelExt;
use crate::image::IconLabel;
use crate::modules::PopupButton;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::{module_impl, spawn};
use color_eyre::Result;
use gtk::{Button, prelude::*};
use gtk::{Label, Orientation};
use serde::Deserialize;
use std::cmp::Ordering;
use std::fmt::Write;
use tokio::sync::mpsc;

const DAY: i64 = 24 * 60 * 60;
const HOUR: i64 = 60 * 60;
const MINUTE: i64 = 60;

#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
struct ProfileState {
    percent: f64,
    charging: Option<bool>,
}

impl PartialOrd for ProfileState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.percent == other.percent {
            match (self.charging, other.charging) {
                (Some(_), Some(_)) | (None, None) => Some(Ordering::Equal),
                (None, Some(_)) => Some(Ordering::Greater),
                (Some(_), None) => Some(Ordering::Less),
            }
        } else {
            self.percent.partial_cmp(&other.percent)
        }
    }
}

impl State for ProfileState {
    fn matches(&self, value: &Self) -> bool {
        match self.charging {
            Some(charging) => {
                charging == value.charging.expect("value should exist")
                    && value.percent <= self.percent
            }
            None => value.percent <= self.percent,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
struct BatteryProfile {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{percentage}%`
    format: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BatteryModule {
    /// The size to render the icon at, in pixels.
    ///
    /// **Default**: `24`
    icon_size: i32,

    // -- Common --
    /// See [layout options](module-level-options#layout)
    #[serde(flatten)]
    layout: LayoutConfig,

    /// Whether to show the icon.
    ///
    /// **Default**: `true`
    show_icon: bool,

    /// Whether to show the label.
    ///
    /// **Default**: `true`
    show_label: bool,

    /// See [profiles](profiles).
    #[serde(flatten)]
    profiles: Profiles<ProfileState, BatteryProfile>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for BatteryModule {
    fn default() -> Self {
        Self {
            icon_size: default::IconSize::Small as i32,
            layout: LayoutConfig::default(),
            show_icon: true,
            show_label: true,
            profiles: Profiles::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

impl Default for BatteryProfile {
    fn default() -> Self {
        Self {
            format: "{percentage}%".to_string(),
        }
    }
}

struct BatteryUiUpdate {
    time_to_full: i64,
    time_to_empty: i64,
    icon_name: String,
    state_name: String,
}

impl Module<Button> for BatteryModule {
    type SendMessage = upower::State;
    type ReceiveMessage = ();

    module_impl!("battery");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();

        let client = context.try_client::<upower::Client>()?;

        spawn(async move {
            let properties = client.state().await?;
            tx.send_update(properties).await;

            let mut rx = client.subscribe();
            while let Ok(properties) = rx.recv().await {
                tx.send_update(properties).await;
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

        let mut manager = self.profiles.attach(&button, move |_button, event| {
            let state = event.state;
            let properties: BatteryUiUpdate = event.data;

            if let Some(l) = &label {
                let time_remaining = if state.charging.expect("should be present on state") {
                    seconds_to_string(properties.time_to_full)
                } else {
                    seconds_to_string(properties.time_to_empty)
                }
                .unwrap_or_default();
                let format = event
                    .profile
                    .format
                    .replace("{percentage}", &state.percent.round().to_string())
                    .replace("{time_remaining}", &time_remaining)
                    .replace("{state}", &properties.state_name);

                l.set_label_escaped(&format);
            }

            if let Some(i) = &icon {
                i.set_label(Some(&format!("icon:{}", properties.icon_name)));
            }
        });

        let rx = context.subscribe();
        rx.recv_glib((), move |(), properties| {
            let percent = properties.percentage;

            let state = properties.state;
            let charging = state == BatteryState::Charging || state == BatteryState::PendingCharge;

            let data = BatteryUiUpdate {
                time_to_full: properties.time_to_full,
                time_to_empty: properties.time_to_empty,
                icon_name: properties.icon_name,
                state_name: state.to_string(),
            };

            manager.update(
                ProfileState {
                    percent,
                    charging: Some(charging),
                },
                data,
            );
        });

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
