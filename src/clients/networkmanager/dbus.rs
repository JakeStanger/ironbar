use color_eyre::Result;
use zbus::proxy;
use zbus::zvariant::{ObjectPath, OwnedValue, Str};

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
pub(super) trait Dbus {
    #[zbus(property)]
    fn all_devices(&self) -> Result<Vec<ObjectPath<'_>>>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device"
)]
pub(super) trait DeviceDbus {
    #[zbus(property)]
    fn device_type(&self) -> Result<DeviceType>;

    #[zbus(property)]
    fn interface(&self) -> Result<Str<'_>>;

    #[zbus(property)]
    fn state(&self) -> Result<DeviceState>;
}

// For reference: https://gitlab.freedesktop.org/NetworkManager/NetworkManager/-/blob/e1a7d5ac062f4f23ce3a6b33c62e856056161ad8/src/libnm-core-public/nm-dbus-interface.h#L212-L253
#[derive(Clone, Debug, Eq, Hash, OwnedValue, PartialEq)]
#[repr(u32)]
pub enum DeviceType {
    Unknown = 0,
    Ethernet = 1,
    Wifi = 2,
    Bluetooth = 5,
    OlpcMesh = 6,
    Wimax = 7,
    Modem = 8,
    Infiniband = 9,
    Bond = 10,
    Vlan = 11,
    Adsl = 12,
    Bridge = 13,
    Team = 15,
    Tun = 16,
    IpTunnel = 17,
    Macvlan = 18,
    Vxlan = 19,
    Veth = 20,
    Macsec = 21,
    Dummy = 22,
    Ppp = 23,
    OvsInterface = 24,
    OvsPort = 25,
    OvsBridge = 26,
    Wpan = 27,
    Lowpan = 28,
    Wireguard = 29,
    WifiP2p = 30,
    Vrf = 31,
    Loopback = 32,
    Hsr = 33,
}

// For reference: https://gitlab.freedesktop.org/NetworkManager/NetworkManager/-/blob/e1a7d5ac062f4f23ce3a6b33c62e856056161ad8/src/libnm-core-public/nm-dbus-interface.h#L501-L538
#[derive(Clone, Debug, OwnedValue, PartialEq)]
#[repr(u32)]
pub enum DeviceState {
    Unknown = 0,
    Unmanaged = 10,
    Unavailable = 20,
    Disconnected = 30,
    Prepare = 40,
    Config = 50,
    NeedAuth = 60,
    IpConfig = 70,
    IpCheck = 80,
    Secondaries = 90,
    Activated = 100,
    Deactivating = 110,
    Failed = 120,
}
