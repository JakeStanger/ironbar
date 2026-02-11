use crate::clients::networkmanager::DeviceState;
use crate::clients::networkmanager::DeviceType;
use crate::clients::networkmanager::state::DeviceTypeData;
use crate::config::{CommonConfig, Profiles, State, default};
use crate::profiles;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct NetworkManagerModule {
    /// The size of the icon for each network device, in pixels.
    pub(super) icon_size: i32,

    /// See [profiles](profiles).
    #[serde(flatten)]
    pub(super) profiles: Profiles<ProfileState, NetworkManagerProfile>,

    /// Any device with a type in this list will not be shown. The type is a string matching
    /// [`DeviceType`] variants (e.g. `"Wifi"`, `"Ethernet", etc.).
    #[serde(default)]
    pub(super) types_blacklist: Vec<DeviceType>,

    /// If not empty, only devices with a type in this list will be shown. The type is a string
    /// matching [`DeviceType`] variants (e.g. `"Wifi"`, `"Ethernet", etc.).
    #[serde(default)]
    pub(super) types_whitelist: Vec<DeviceType>,

    /// Any device whose interface name is in this list will not be shown.
    #[serde(default)]
    pub(super) interface_blacklist: Vec<String>,

    /// If not empty, only devices whose interface name is in this list will be shown.
    #[serde(default)]
    pub(super) interface_whitelist: Vec<String>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}
impl NetworkManagerModule {
    pub(super) fn get_tooltip(
        &self,
        device: &crate::clients::networkmanager::state::Device,
    ) -> String {
        let mut tooltip = device.interface.clone();
        if let Some(ip) = &device.ip4_config {
            for x in &ip.address_data {
                tooltip.push('\n');
                tooltip.push_str(&x.address);
                tooltip.push('/');
                tooltip.push_str(&x.prefix.to_string());
            }
        }
        if let DeviceTypeData::Wireless(wireless) = &device.device_type_data
            && let Some(connection) = &wireless.active_access_point
        {
            tooltip.push('\n');
            tooltip.push_str(&String::from_utf8_lossy(&connection.ssid));
        };

        tooltip
    }

    pub(super) fn get_profile_state(
        &self,
        device: &crate::clients::networkmanager::state::Device,
    ) -> Option<ProfileState> {
        fn whitelisted<T: PartialEq>(list: &[T], x: &T) -> bool {
            list.is_empty() || list.contains(x)
        }

        let type_whitelisted = whitelisted(&self.types_whitelist, &device.device_type);
        let interface_whitelisted = whitelisted(&self.interface_whitelist, &device.interface);
        let type_blacklisted = self.types_blacklist.contains(&device.device_type);
        let interface_blacklisted = self.interface_blacklist.contains(&device.interface);

        if !type_whitelisted || !interface_whitelisted || type_blacklisted || interface_blacklisted
        {
            return None;
        }

        let state = ConnectionState::from(device.state);

        let state = match device.device_type {
            DeviceType::Wifi => match state {
                ConnectionState::Acquiring => ProfileState::Wifi(WifiConnectionState::Acquiring),
                ConnectionState::Disconnected => {
                    ProfileState::Wifi(WifiConnectionState::Disconnected)
                }
                ConnectionState::Connected => match &device.device_type_data {
                    DeviceTypeData::Wireless(wireless) => match &wireless.active_access_point {
                        Some(connection) => ProfileState::Wifi(WifiConnectionState::Connected {
                            signal_strength: connection.strength,
                        }),
                        None => ProfileState::Wifi(WifiConnectionState::Disconnected),
                    },
                    _ => ProfileState::Unknown,
                },
            },
            DeviceType::Modem | DeviceType::Wimax => match state {
                ConnectionState::Acquiring => ProfileState::Cellular(ConnectionState::Acquiring),
                ConnectionState::Disconnected => {
                    ProfileState::Cellular(ConnectionState::Disconnected)
                }
                ConnectionState::Connected => ProfileState::Cellular(ConnectionState::Connected),
            },
            DeviceType::Wireguard
            | DeviceType::Tun
            | DeviceType::IpTunnel
            | DeviceType::Vxlan
            | DeviceType::Macsec => match state {
                ConnectionState::Acquiring => ProfileState::Vpn(ConnectionState::Acquiring),
                ConnectionState::Disconnected => ProfileState::Vpn(ConnectionState::Disconnected),
                ConnectionState::Connected => ProfileState::Vpn(ConnectionState::Connected),
            },
            _ => match state {
                ConnectionState::Acquiring => ProfileState::Wired(ConnectionState::Acquiring),
                ConnectionState::Disconnected => ProfileState::Wired(ConnectionState::Disconnected),
                ConnectionState::Connected => ProfileState::Wired(ConnectionState::Connected),
            },
        };

        Some(state)
    }
}

impl Default for NetworkManagerModule {
    fn default() -> Self {
        Self {
            icon_size: default::IconSize::Small as i32,
            common: Some(CommonConfig::default()),
            profiles: Profiles::default(),
            types_blacklist: Vec::new(),
            types_whitelist: Vec::new(),
            interface_blacklist: Vec::new(),
            interface_whitelist: Vec::new(),
        }
    }
}

#[derive(Default, Debug, Deserialize, Clone, Copy, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Acquiring,
    Connected,
}

impl From<DeviceState> for ConnectionState {
    fn from(state: DeviceState) -> Self {
        match state {
            DeviceState::Unknown
            | DeviceState::Unmanaged
            | DeviceState::Unavailable
            | DeviceState::Deactivating
            | DeviceState::Failed
            | DeviceState::Disconnected => ConnectionState::Disconnected,
            DeviceState::Prepare
            | DeviceState::Config
            | DeviceState::NeedAuth
            | DeviceState::IpConfig
            | DeviceState::IpCheck
            | DeviceState::Secondaries => ConnectionState::Acquiring,
            DeviceState::Activated => ConnectionState::Connected,
        }
    }
}

#[derive(Default, Debug, Deserialize, Clone, Copy, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WifiConnectionState {
    #[default]
    Disconnected,
    Acquiring,
    Connected {
        /// The signal strength of the wifi connection, from 0 to 100.
        #[serde(default = "default_signal_strength")]
        signal_strength: u8,
    },
}

fn default_signal_strength() -> u8 {
    // 255 so it matches any signal strength.
    255
}

#[derive(Default, Debug, Deserialize, Clone, Copy, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProfileState {
    #[default]
    Unknown,
    Wired(ConnectionState),
    Wifi(WifiConnectionState),
    Cellular(ConnectionState),
    Vpn(ConnectionState),
}

impl State for ProfileState {
    fn matches(&self, value: &Self) -> bool {
        match (self, value) {
            (ProfileState::Wifi(self_state), ProfileState::Wifi(value_state)) => {
                match (self_state, value_state) {
                    (
                        WifiConnectionState::Connected {
                            signal_strength: self_signal,
                        },
                        WifiConnectionState::Connected {
                            signal_strength: value_signal,
                        },
                    ) => value_signal <= self_signal,
                    (WifiConnectionState::Acquiring, WifiConnectionState::Acquiring) => true,
                    (WifiConnectionState::Disconnected, WifiConnectionState::Disconnected) => true,
                    _ => false,
                }
            }
            _ => self == value,
        }
    }
}

#[derive(Clone, Default, Deserialize, Debug)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct NetworkManagerProfile {
    /// The configuration for the icons used to represent network devices.
    pub icon: String,
}

pub(super) fn default_profiles() -> Profiles<ProfileState, NetworkManagerProfile> {
    use ProfileState::*;

    let i = |icon: &str| NetworkManagerProfile {
        icon: icon.to_string(),
    };
    profiles!(
        "wired_disconnected": Wired(ConnectionState::Disconnected) => i(""),
        "wired_acquiring": Wired(ConnectionState::Acquiring) => i("icon:network-wired-acquiring-symbolic"),
        "wired_connected": Wired(ConnectionState::Connected) => i("icon:network-wired-symbolic"),

        "wifi_disconnected": Wifi(WifiConnectionState::Disconnected) => i(""),
        "wifi_acquiring": Wifi(WifiConnectionState::Acquiring) => i("icon:network-wireless-acquiring-symbolic"),

        // Thresholds based on GNOME's wifi icon thresholds: https://gitlab.gnome.org/GNOME/gnome-shell/-/blob/449a4e354af82a2412cfb1ee2fe26451631aeae6/js/ui/status/network.js#L46-57
        "wifi_connected_none": Wifi(WifiConnectionState::Connected{signal_strength:20}) => i("icon:network-wireless-signal-none-symbolic"),
        "wifi_connected_weak": Wifi(WifiConnectionState::Connected{signal_strength:40}) => i("icon:network-wireless-signal-weak-symbolic"),
        "wifi_connected_ok": Wifi(WifiConnectionState::Connected{signal_strength:50}) => i("icon:network-wireless-signal-ok-symbolic"),
        "wifi_connected_good": Wifi(WifiConnectionState::Connected{signal_strength:80}) => i("icon:network-wireless-signal-good-symbolic"),
        "wifi_connected_excellent": Wifi(WifiConnectionState::Connected{signal_strength:100}) => i("icon:network-wireless-signal-excellent-symbolic"),

        "cellular_disconnected": Cellular(ConnectionState::Disconnected) => i(""),
        "cellular_acquiring": Cellular(ConnectionState::Acquiring) => i("icon:network-cellular-acquiring-symbolic"),
        "cellular_connected": Cellular(ConnectionState::Connected) => i("icon:network-cellular-connected-symbolic"),

        "vpn_disconnected": Vpn(ConnectionState::Disconnected) => i(""),
        "vpn_acquiring": Vpn(ConnectionState::Acquiring) => i("icon:network-vpn-acquiring-symbolic"),
        "vpn_connected": Vpn(ConnectionState::Connected) => i("icon:network-vpn-symbolic"),

        "unknown": Unknown => i("icon:dialog-question-symbolic")
    )
}
