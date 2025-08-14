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
    fn active_connections(&self) -> Result<Vec<ObjectPath<'_>>>;

    #[zbus(property)]
    fn devices(&self) -> Result<Vec<ObjectPath<'_>>>;

    // #[zbus(property)]
    // fn networking_enabled(&self) -> Result<bool>;

    // #[zbus(property)]
    // fn primary_connection(&self) -> Result<ObjectPath>;

    // #[zbus(property)]
    // fn primary_connection_type(&self) -> Result<Str>;

    // #[zbus(property)]
    // fn wireless_enabled(&self) -> Result<bool>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
pub(super) trait ActiveConnectionDbus {
    // #[zbus(property)]
    // fn connection(&self) -> Result<ObjectPath>;

    // #[zbus(property)]
    // fn default(&self) -> Result<bool>;

    // #[zbus(property)]
    // fn default6(&self) -> Result<bool>;

    #[zbus(property)]
    fn devices(&self) -> Result<Vec<ObjectPath<'_>>>;

    // #[zbus(property)]
    // fn id(&self) -> Result<Str>;

    #[zbus(property)]
    fn type_(&self) -> Result<Str<'_>>;

    // #[zbus(property)]
    // fn uuid(&self) -> Result<Str>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device"
)]
pub(super) trait DeviceDbus {
    // #[zbus(property)]
    // fn active_connection(&self) -> Result<ObjectPath>;

    #[zbus(property)]
    fn device_type(&self) -> Result<DeviceType>;

    #[zbus(property)]
    fn state(&self) -> Result<DeviceState>;
}

#[derive(Clone, Debug, Eq, Hash, OwnedValue, PartialEq)]
#[repr(u32)]
pub(super) enum DeviceType {
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

#[derive(Clone, Debug, OwnedValue, PartialEq)]
#[repr(u32)]
pub(super) enum DeviceState {
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

impl DeviceState {
    pub(super) fn is_enabled(&self) -> bool {
        !matches!(
            self,
            DeviceState::Unknown | DeviceState::Unmanaged | DeviceState::Unavailable,
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Device<'l> {
    pub object_path: ObjectPath<'l>,
    pub type_: DeviceType,
}
