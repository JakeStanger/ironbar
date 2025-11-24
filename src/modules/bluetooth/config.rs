use serde::Deserialize;

use crate::{
    clients::bluetooth::{BluetoothDevice, BluetoothDeviceStatus, BluetoothState},
    config::CommonConfig,
};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BluetoothModule {
    /// Format strings for on-bar button.
    pub format: FormatConfig,

    /// Popup related configuration.
    pub popup: PopupConfig,

    /// Values of `{adapter_status}` formatting token.
    pub adapter_status: AdapterStatus,

    /// Values of `{device_status}` formatting token.
    pub device_status: DeviceStatus,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    pub icon_size: i32,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for BluetoothModule {
    fn default() -> Self {
        Self {
            format: FormatConfig::default(),
            popup: PopupConfig::default(),
            adapter_status: AdapterStatus::default(),
            device_status: DeviceStatus::default(),
            icon_size: 32,
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct FormatConfig {
    /// Format string to use for the widget button when bluetooth adapter not found.
    ///
    /// **Default**: `""`
    pub not_found: String,

    /// Format string to use for the widget button when bluetooth adapter is disabled.
    ///
    /// **Default**: `" Off"`
    pub disabled: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled but no devices are connected.
    ///
    /// **Default**: `" On"`
    pub enabled: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled and a device is connected.
    ///
    /// **Default**: `" {device_alias}"`
    pub connected: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled, a device is connected and `{device_battery_percent}` is available.
    ///
    /// **Default**: `" {device_alias} • {device_battery_percent}%"`
    pub connected_battery: String,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            not_found: String::new(),
            disabled: " Off".to_string(),
            enabled: " On".to_string(),
            connected: " {device_alias}".to_string(),
            connected_battery: " {device_alias} • {device_battery_percent}%".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct PopupConfig {
    /// The maximum number of pixels the window can reach before scrolling.
    /// Leave blank to allow the popup to grow indefinitely.
    ///
    /// **Default**: `Some(330)`
    pub max_height: Option<i32>,

    /// Format string to use for the header of popup window.
    ///
    /// **Default**: `" Enable Bluetooth"`
    pub header: String,

    /// Format string to use for the message that is displayed when the adapter is not found or disabled.
    ///
    /// **Default**: `"{adapter_status}"`
    pub disabled: String,

    /// Device box related configuration
    pub device: PopupDeviceConfig,
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            max_height: Some(330),
            header: " Enable Bluetooth".to_string(),
            disabled: "{adapter_status}".to_string(),
            device: PopupDeviceConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct AdapterStatus {
    /// The value of `{adapter_status}` formatting token when adapter is enabling.
    ///
    /// **Default**: `"Enabling Bluetooth..."`
    pub enabling: String,

    /// The value of `{adapter_status}` formatting token when adapter is enabled.
    ///
    /// **Default**: `"Bluetooth enabled"`
    pub enabled: String,

    /// The value of `{adapter_status}` formatting token when adapter is disabling.
    ///
    /// **Default**: `"Disabling Bluetooth..."`
    pub disabling: String,

    /// The value of `{adapter_status}` formatting token when adapter is disabled.
    ///
    /// **Default**: `"Bluetooth disabled"`
    pub disabled: String,

    /// The value of `{adapter_status}` formatting token when adapter not found.
    ///
    /// **Default**: `"No Bluetooth adapters found"`
    pub not_found: String,
}

impl Default for AdapterStatus {
    fn default() -> Self {
        Self {
            enabling: "Enabling Bluetooth...".to_string(),
            enabled: "Bluetooth enabled".to_string(),
            disabling: "Disabling Bluetooth...".to_string(),
            disabled: "Bluetooth disabled".to_string(),
            not_found: "No Bluetooth adapters found".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct DeviceStatus {
    /// The value of `{device_status}` formatting token when device is connecting.
    ///
    /// **Default**: `"Connecting..."`
    pub connecting: String,

    /// The value of `{device_status}` formatting token when device is connected.
    ///
    /// **Default**: `"Connected"`
    pub connected: String,

    /// The value of `{device_status}` formatting token when device is disconnecting.
    ///
    /// **Default**: `"Disconnecting..."`
    pub disconnecting: String,

    /// The value of `{device_status}` formatting token when device is disconnected.
    ///
    /// **Default**: `"Disconnect"`
    pub disconnected: String,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            connecting: "Connecting...".to_string(),
            connected: "Connected".to_string(),
            disconnecting: "Disconnecting...".to_string(),
            disconnected: "Disconnected".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct PopupDeviceConfig {
    /// Format string to use for the header of device box.
    ///
    /// **Default**: `"{device_alias}"`
    pub header: String,

    /// Format string to use for the header of device box when `{device_battery_percent}` is available.
    ///
    /// **Default**: `"{device_alias}"`
    pub header_battery: String,

    /// Format string to use for the footer of device box.
    ///
    /// **Default**: `"{device_status}"`
    pub footer: String,

    /// Format string to use for the footer of device box when `{device_battery_percent}` is available.
    ///
    /// **Default**: `"{device_status} • Battery {device_battery_percent}%"`
    pub footer_battery: String,
}

impl Default for PopupDeviceConfig {
    fn default() -> Self {
        Self {
            header: "{device_alias}".to_string(),
            header_battery: "{device_alias}".to_string(),
            footer: "{device_status}".to_string(),
            footer_battery: "{device_status} • Battery {device_battery_percent}%".to_string(),
        }
    }
}

impl BluetoothModule {
    pub fn format_adapter(
        state: &BluetoothState,
        adapter_status: &AdapterStatus,
        str: &str,
    ) -> String {
        let status = match state {
            BluetoothState::Enabling => &adapter_status.enabling,
            BluetoothState::Enabled { .. } => &adapter_status.enabled,
            BluetoothState::Disabling => &adapter_status.disabling,
            BluetoothState::Disabled => &adapter_status.disabled,
            BluetoothState::NotFound => &adapter_status.not_found,
        };

        str.replace("{adapter_status}", status)
    }

    pub fn format_device(
        device: &BluetoothDevice,
        device_status: &DeviceStatus,
        str: &str,
    ) -> String {
        let status = match device.status {
            BluetoothDeviceStatus::Connecting => &device_status.connecting,
            BluetoothDeviceStatus::Connected => &device_status.connected,
            BluetoothDeviceStatus::Disconnecting => &device_status.disconnecting,
            BluetoothDeviceStatus::Disconnected => &device_status.disconnected,
        };

        let battery_percent = if let Some(percent) = device.battery_percent {
            format!("{percent}")
        } else {
            String::new()
        };

        str.replace("{device_address}", &format!("{}", &device.address))
            .replace("{device_status}", status)
            .replace("{device_alias}", &device.alias)
            .replace("{device_battery_percent}", &battery_percent)
    }

    pub fn replace_device(
        state: &BluetoothDevice,
        device_status: &DeviceStatus,
        device: &PopupDeviceConfig,
    ) -> PopupDeviceConfig {
        PopupDeviceConfig {
            header: Self::format_device(state, device_status, &device.header),
            header_battery: Self::format_device(state, device_status, &device.header_battery),
            footer: Self::format_device(state, device_status, &device.footer),
            footer_battery: Self::format_device(state, device_status, &device.footer_battery),
        }
    }
}
