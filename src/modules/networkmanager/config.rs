use crate::config::{Profiles, State};
use crate::profiles;
use serde::Deserialize;

#[derive(Default, Debug, Deserialize, Clone, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum WiredState {
    #[default]
    Disconnected,
    Acquiring,
    Connected,
}

#[derive(Default, Debug, Deserialize, Clone, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum WifiState {
    #[default]
    Disconnected,
    Acquiring,
    Connected {
        /// The signal strength of the wifi connection, from 0 to 100.
        signal_strength: u8,
    },
}

#[derive(Default, Debug, Deserialize, Clone, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum CellularState {
    #[default]
    Disconnected,
    Acquiring,
    Connected,
}

#[derive(Default, Debug, Deserialize, Clone, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum VpnState {
    #[default]
    Disconnected,
    Acquiring,
    Connected,
}

#[derive(Default, Debug, Deserialize, Clone, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum ProfileState {
    #[default]
    Unknown,
    Wired(WiredState),
    Wifi(WifiState),
    Cellular(CellularState),
    Vpn(VpnState),
}

impl State for ProfileState {
    fn matches(&self, value: &Self) -> bool {
        match (self, value) {
            (ProfileState::Wifi(self_state), ProfileState::Wifi(value_state)) => {
                match (self_state, value_state) {
                    (
                        WifiState::Connected {
                            signal_strength: self_signal,
                        },
                        WifiState::Connected {
                            signal_strength: value_signal,
                        },
                    ) => value_signal <= self_signal,
                    (WifiState::Acquiring, WifiState::Acquiring) => true,
                    (WifiState::Disconnected, WifiState::Disconnected) => true,
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
        "wired_disconnected": Wired(WiredState::Disconnected) => i(""),
        "wired_acquiring": Wired(WiredState::Acquiring) => i("icon:network-wired-acquiring-symbolic"),
        "wired_connected": Wired(WiredState::Connected) => i("icon:network-wired-symbolic"),

        "wifi_disconnected": Wifi(WifiState::Disconnected) => i(""),
        "wifi_acquiring": Wifi(WifiState::Acquiring) => i("icon:network-wireless-acquiring-symbolic"),

        // Thresholds based on GNOME's wifi icon thresholds: https://gitlab.gnome.org/GNOME/gnome-shell/-/blob/449a4e354af82a2412cfb1ee2fe26451631aeae6/js/ui/status/network.js#L46-57
        "wifi_connected_none": Wifi(WifiState::Connected { signal_strength: 20 }) => i("icon:network-wireless-signal-none-symbolic"),
        "wifi_connected_weak": Wifi(WifiState::Connected { signal_strength: 40 }) => i("icon:network-wireless-signal-weak-symbolic"),
        "wifi_connected_ok": Wifi(WifiState::Connected { signal_strength: 50 }) => i("icon:network-wireless-signal-ok-symbolic"),
        "wifi_connected_good": Wifi(WifiState::Connected { signal_strength: 80 }) => i("icon:network-wireless-signal-good-symbolic"),
        "wifi_connected_excellent": Wifi(WifiState::Connected { signal_strength: 100 }) => i("icon:network-wireless-signal-excellent-symbolic"),

        "cellular_disconnected": Cellular(CellularState::Disconnected) => i(""),
        "cellular_acquiring": Cellular(CellularState::Acquiring) => i("icon:network-cellular-acquiring-symbolic"),
        "cellular_connected": Cellular(CellularState::Connected) => i("icon:network-cellular-connected-symbolic"),

        "vpn_disconnected": Vpn(VpnState::Disconnected) => i(""),
        "vpn_acquiring": Vpn(VpnState::Acquiring) => i("icon:network-vpn-acquiring-symbolic"),
        "vpn_connected": Vpn(VpnState::Connected) => i("icon:network-vpn-symbolic"),

        "unknown": Unknown => i("icon:dialog-question-symbolic")
    )
}
