/// Originally taken from `upower-dbus` crate
/// <https://github.com/pop-os/upower-dbus/blob/main/LICENSE>
///  Copyright 2021 System76 <info@system76.com>
///  SPDX-License-Identifier: MPL-2.0
use zbus::proxy;
use zbus::zvariant::{OwnedValue, Value};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, OwnedValue)]
#[repr(u32)]
pub enum BatteryState {
    #[default]
    Unknown = 0,
    Charging = 1,
    Discharging = 2,
    Empty = 3,
    FullyCharged = 4,
    PendingCharge = 5,
    PendingDischarge = 6,
}

impl From<u32> for BatteryState {
    fn from(number: u32) -> Self {
        match number {
            n if n == BatteryState::Charging as u32 => BatteryState::Charging,
            n if n == BatteryState::Discharging as u32 => BatteryState::Discharging,
            n if n == BatteryState::Empty as u32 => BatteryState::Empty,
            n if n == BatteryState::FullyCharged as u32 => BatteryState::FullyCharged,
            n if n == BatteryState::PendingCharge as u32 => BatteryState::PendingCharge,
            n if n == BatteryState::PendingDischarge as u32 => BatteryState::PendingCharge,
            _ => BatteryState::Unknown,
        }
    }
}

impl TryFrom<&zbus::zvariant::Value<'_>> for BatteryState {
    type Error = zbus::zvariant::Error;

    fn try_from(value: &Value<'_>) -> Result<Self, Self::Error> {
        let value = value.downcast_ref::<u32>()?;
        Ok(value.into())
    }
}

#[derive(Debug, Copy, Clone, OwnedValue)]
#[repr(u32)]
pub enum BatteryType {
    Unknown = 0,
    LinePower = 1,
    Battery = 2,
    Ups = 3,
    Monitor = 4,
    Mouse = 5,
    Keyboard = 6,
    Pda = 7,
    Phone = 8,
}

#[derive(Debug, Copy, Clone, OwnedValue)]
#[repr(u32)]
pub enum BatteryLevel {
    Unknown = 0,
    None = 1,
    Low = 3,
    Critical = 4,
    Normal = 6,
    High = 7,
    Full = 8,
}

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower",
    assume_defaults = false
)]
pub trait Device {
    #[zbus(property)]
    fn battery_level(&self) -> zbus::Result<BatteryLevel>;

    #[zbus(property)]
    fn capacity(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn energy(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn energy_empty(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn energy_full(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn energy_full_design(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn has_history(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn has_statistics(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn is_present(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn is_rechargeable(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn luminosity(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn model(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn native_path(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn online(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn power_supply(&self) -> zbus::Result<bool>;

    fn refresh(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn serial(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<BatteryState>;

    #[zbus(property)]
    fn temperature(&self) -> zbus::Result<f64>;

    #[zbus(property, name = "Type")]
    fn type_(&self) -> zbus::Result<BatteryType>;

    #[zbus(property)]
    fn vendor(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn voltage(&self) -> zbus::Result<f64>;
}

#[proxy(interface = "org.freedesktop.UPower", assume_defaults = true)]
pub trait UPower {
    /// EnumerateDevices method
    fn enumerate_devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// GetCriticalAction method
    fn get_critical_action(&self) -> zbus::Result<String>;

    /// GetDisplayDevice method
    #[zbus(object = "Device")]
    fn get_display_device(&self);

    /// DeviceAdded signal
    #[zbus(signal)]
    fn device_added(&self, device: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    /// DeviceRemoved signal
    #[zbus(signal)]
    fn device_removed(&self, device: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    /// DaemonVersion property
    #[zbus(property)]
    fn daemon_version(&self) -> zbus::Result<String>;

    /// LidIsClosed property
    #[zbus(property)]
    fn lid_is_closed(&self) -> zbus::Result<bool>;

    /// LidIsPresent property
    #[zbus(property)]
    fn lid_is_present(&self) -> zbus::Result<bool>;

    /// OnBattery property
    #[zbus(property)]
    fn on_battery(&self) -> zbus::Result<bool>;
}
