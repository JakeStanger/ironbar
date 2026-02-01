#[cfg(not(feature = "battery.test"))]
mod client;
mod dbus;
#[cfg(feature = "battery.test")]
mod test_client;

use crate::register_fallible_client;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use zbus::zvariant::OwnedValue;

pub use dbus::BatteryState;

#[cfg(not(feature = "battery.test"))]
pub use client::Client;

#[cfg(feature = "battery.test")]
pub use test_client::Client;

#[derive(Clone, Debug, Default)]
pub struct State {
    /// Battery charge percentage.
    pub percentage: f64,
    /// Battery state (charging, discharging, ...)
    pub state: BatteryState,
    /// Icon to display for the state and percentage.
    pub icon_name: String,
    /// Number of seconds until full charge, if charging.
    pub time_to_full: i64,
    /// Number of seconds until empty, if discharging.
    pub time_to_empty: i64,
}

impl TryFrom<HashMap<String, OwnedValue>> for State {
    type Error = zbus::zvariant::Error;

    fn try_from(properties: HashMap<String, OwnedValue>) -> Result<Self, Self::Error> {
        Ok(Self {
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

register_fallible_client!(Client, upower);
