use super::device::DeviceProxy;
/// Originally taken from `upower-dbus` crate
/// https://github.com/pop-os/upower-dbus/blob/main/LICENSE
// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0
use zbus::proxy;

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
