use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct Icons {
    pub wired: IconsWired,
    pub wifi: IconsWifi,
    pub cellular: IconsCellular,
    pub vpn: IconsVpn,
    pub unknown: String,
}
impl Default for Icons {
    fn default() -> Self {
        Self {
            wired: IconsWired::default(),
            wifi: IconsWifi::default(),
            cellular: IconsCellular::default(),
            vpn: IconsVpn::default(),
            unknown: "icon:dialog-question-symbolic".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsWired {
    pub connected: String,
    pub acquiring: String,
    pub disconnected: String,
}
impl Default for IconsWired {
    fn default() -> Self {
        Self {
            connected: "icon:network-wired-symbolic".to_string(),
            acquiring: "icon:network-wired-acquiring-symbolic".to_string(),
            disconnected: "".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsWifi {
    pub levels: Vec<String>,
    pub acquiring: String,
    pub disconnected: String,
}

impl Default for IconsWifi {
    fn default() -> Self {
        Self {
            levels: vec![
                "icon:network-wireless-signal-none-symbolic".to_string(),
                "icon:network-wireless-signal-weak-symbolic".to_string(),
                "icon:network-wireless-signal-ok-symbolic".to_string(),
                "icon:network-wireless-signal-good-symbolic".to_string(),
                "icon:network-wireless-signal-excellent-symbolic".to_string(),
            ],
            acquiring: "icon:network-wireless-acquiring-symbolic".to_string(),
            disconnected: "".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsCellular {
    pub connected: String,
    pub acquiring: String,
    pub disconnected: String,
}
impl Default for IconsCellular {
    fn default() -> Self {
        Self {
            connected: "icon:network-cellular-connected-symbolic".to_string(),
            acquiring: "icon:network-cellular-acquiring-symbolic".to_string(),
            disconnected: "".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct IconsVpn {
    pub connected: String,
    pub acquiring: String,
    pub disconnected: String,
}
impl Default for IconsVpn {
    fn default() -> Self {
        Self {
            connected: "icon:network-vpn-symbolic".to_string(),
            acquiring: "icon:network-vpn-acquiring-symbolic".to_string(),
            disconnected: "".to_string(),
        }
    }
}
