use color_eyre::Result;
use zbus::dbus_proxy;
use zbus::zvariant::{ObjectPath, OwnedValue, Str};

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait Dbus {
    #[dbus_proxy(property)]
    fn active_connections(&self) -> Result<Vec<ObjectPath>>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<ObjectPath>>;

    // #[dbus_proxy(property)]
    // fn networking_enabled(&self) -> Result<bool>;

    // #[dbus_proxy(property)]
    // fn primary_connection(&self) -> Result<ObjectPath>;

    // #[dbus_proxy(property)]
    // fn primary_connection_type(&self) -> Result<Str>;

    // #[dbus_proxy(property)]
    // fn wireless_enabled(&self) -> Result<bool>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
trait ActiveConnectionDbus {
    // #[dbus_proxy(property)]
    // fn connection(&self) -> Result<ObjectPath>;

    // #[dbus_proxy(property)]
    // fn default(&self) -> Result<bool>;

    // #[dbus_proxy(property)]
    // fn default6(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<ObjectPath>>;

    // #[dbus_proxy(property)]
    // fn id(&self) -> Result<Str>;

    #[dbus_proxy(property)]
    fn type_(&self) -> Result<Str>;

    // #[dbus_proxy(property)]
    // fn uuid(&self) -> Result<Str>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device"
)]
trait DeviceDbus {
    // #[dbus_proxy(property)]
    // fn active_connection(&self) -> Result<ObjectPath>;

    #[dbus_proxy(property)]
    fn device_type(&self) -> Result<DeviceType>;

    #[dbus_proxy(property)]
    fn state(&self) -> Result<DeviceState>;
}

#[derive(Clone, Debug, OwnedValue, PartialEq)]
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
