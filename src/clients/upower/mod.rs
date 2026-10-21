#[cfg(not(feature = "battery.test"))]
mod client;
mod dbus;
#[cfg(feature = "battery.test")]
mod test_client;

use crate::register_fallible_client;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use zbus::OwnedVariant;

pub use dbus::{BatteryLevel, BatteryState, BatteryType};

#[cfg(not(feature = "battery.test"))]
pub use client::Client;

#[cfg(feature = "battery.test")]
pub use test_client::Client;

#[derive(Clone, Debug, Default)]
pub struct State {
    /// Battery charge percentage.
    pub percentage: f64,
    /// Battery level name (Unknown, None, Low, Critical, Normal, High, Full)
    pub level: BatteryLevel,
    /// Battery state (charging, discharging, ...)
    pub state: BatteryState,
    /// The type of battery connected.
    pub battery_type: BatteryType,
    /// Icon to display for the state and percentage.
    pub icon_name: String,
    /// Number of seconds until full charge, if charging.
    pub time_to_full: i64,
    /// Number of seconds until empty, if discharging.
    pub time_to_empty: i64,
}

impl TryFrom<HashMap<String, OwnedVariant>> for State {
    type Error = zbus::Error;

    fn try_from(properties: HashMap<String, OwnedVariant>) -> Result<Self, Self::Error> {
        Ok(Self {
            percentage: properties["Percentage"].downcast_ref::<f64>()?,
            level: properties["BatteryLevel"].downcast_ref::<BatteryLevel>()?,
            state: properties["State"].downcast_ref::<BatteryState>()?,
            time_to_full: properties["TimeToFull"].downcast_ref::<i64>()?,
            time_to_empty: properties["TimeToEmpty"].downcast_ref::<i64>()?,
            icon_name: properties["IconName"].downcast_ref::<&str>()?.to_string(),
            battery_type: properties["Type"].downcast_ref::<BatteryType>()?,
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

impl Display for BatteryLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BatteryLevel::Unknown => "Unknown",
                BatteryLevel::None => "None",
                BatteryLevel::Low => "Low",
                BatteryLevel::Critical => "Critical",
                BatteryLevel::Normal => "Normal",
                BatteryLevel::High => "High",
                BatteryLevel::Full => "Full",
            }
        )
    }
}

impl Display for BatteryType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BatteryType::Unknown => "Unknown",
                BatteryType::LinePower => "Line power",
                BatteryType::Battery => "Battery",
                BatteryType::Ups => "UPS",
                BatteryType::Monitor => "Monitor",
                BatteryType::Mouse => "Mouse",
                BatteryType::Keyboard => "Keyboard",
                BatteryType::Pda => "PDA",
                BatteryType::Phone => "Phone",
            }
        )
    }
}

register_fallible_client!(Client, upower);
