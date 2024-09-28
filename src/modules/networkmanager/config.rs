use serde::Deserialize;

macro_rules! default_function {
    ($(($name:ident, $default:expr),)*) => {
        $(
            fn $name() -> String {
                ($default).to_string()
            }
        )*
    };
}

#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsConfig {
    #[serde(default)]
    pub wired: IconsConfigWired,
    #[serde(default)]
    pub wifi: IconsConfigWifi,
    #[serde(default)]
    pub cellular: IconsConfigCellular,
    #[serde(default)]
    pub vpn: IconsConfigVpn,

    #[serde(default = "default_unknown")]
    pub unknown: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsConfigWired {
    #[serde(default = "default_wired_connected")]
    pub connected: String,
    #[serde(default = "default_wired_acquiring")]
    pub acquiring: String,
    #[serde(default = "default_wired_disconnected")]
    pub disconnected: String,
}
impl Default for IconsConfigWired {
    fn default() -> Self {
        Self {
            connected: default_wired_connected(),
            acquiring: default_wired_acquiring(),
            disconnected: default_wired_disconnected(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsConfigWifi {
    #[serde(default = "default_wifi_levels")]
    pub levels: Vec<String>,
    #[serde(default = "default_wifi_acquiring")]
    pub acquiring: String,
    #[serde(default = "default_wifi_disconnected")]
    pub disconnected: String,
}

impl Default for IconsConfigWifi {
    fn default() -> Self {
        Self {
            levels: default_wifi_levels(),
            acquiring: default_wifi_acquiring(),
            disconnected: default_wifi_disconnected(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsConfigCellular {
    #[serde(default = "default_cellular_connected")]
    pub connected: String,
    #[serde(default = "default_cellular_acquiring")]
    pub acquiring: String,
    #[serde(default = "default_cellular_disconnected")]
    pub disconnected: String,
}
impl Default for IconsConfigCellular {
    fn default() -> Self {
        Self {
            connected: default_cellular_connected(),
            acquiring: default_cellular_acquiring(),
            disconnected: default_cellular_disconnected(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsConfigVpn {
    #[serde(default = "default_vpn_connected")]
    pub connected: String,
    #[serde(default = "default_vpn_acquiring")]
    pub acquiring: String,
    #[serde(default = "default_vpn_disconnected")]
    pub disconnected: String,
}
impl Default for IconsConfigVpn {
    fn default() -> Self {
        Self {
            connected: default_vpn_connected(),
            acquiring: default_vpn_acquiring(),
            disconnected: default_vpn_disconnected(),
        }
    }
}

pub fn default_wifi_levels() -> Vec<String> {
    vec![
        "icon:network-wireless-signal-none-symbolic".to_string(),
        "icon:network-wireless-signal-weak-symbolic".to_string(),
        "icon:network-wireless-signal-ok-symbolic".to_string(),
        "icon:network-wireless-signal-good-symbolic".to_string(),
        "icon:network-wireless-signal-excellent-symbolic".to_string(),
    ]
}

default_function! {
    (default_wired_connected,  "icon:network-wired-symbolic"),
    (default_wired_acquiring,  "icon:network-wired-acquiring-symbolic"),
    (default_wired_disconnected,  ""),

    (default_wifi_acquiring, "icon:network-wireless-acquiring-symbolic"),
    (default_wifi_disconnected, ""),

    (default_cellular_connected,"icon:network-cellular-connected-symbolic"),
    (default_cellular_acquiring,"icon:network-cellular-acquiring-symbolic"),
    (default_cellular_disconnected,""),

    (default_vpn_connected, "icon:network-vpn-symbolic"),
    (default_vpn_acquiring, "icon:network-vpn-acquiring-symbolic"),
    (default_vpn_disconnected, ""),

    (default_unknown, "icon:dialog-question-symbolic"),
}
