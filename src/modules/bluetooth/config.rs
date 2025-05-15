use serde::Deserialize;

use crate::{
    clients::bluetooth::{BluetoothDevice, BluetoothDeviceStatus, BluetoothState},
    config::CommonConfig,
};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BluetoothModule {
    /// Format strings for on-bar button
    #[serde(default)]
    pub format: FormatConfig,

    /// Popup related configuration
    #[serde(default)]
    pub popup: PopupConfig,

    /// Values of `{adapter_status}` formatting token.
    #[serde(default)]
    pub adapter_status: AdapterStatus,

    /// Values of `{device_status}` formatting token.
    #[serde(default)]
    pub device_status: DeviceStatus,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FormatConfig {
    /// Format string to use for the widget button when bluetooth adapter not found.
    ///
    /// **Default**: `""`
    #[serde(default = "default_format_not_found")]
    pub not_found: String,

    /// Format string to use for the widget button when bluetooth adapter is disabled.
    ///
    /// **Default**: `" Off"`
    #[serde(default = "default_format_disabled")]
    pub disabled: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled but no devices are connected.
    ///
    /// **Default**: `" On"`
    #[serde(default = "default_format_enabled")]
    pub enabled: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled and a device is connected.
    ///
    /// **Default**: `" {device_alias}"`
    #[serde(default = "default_format_connected")]
    pub connected: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled, a device is connected and `{device_battery_percent}` is available.
    ///
    /// **Default**: `" {device_alias} • {device_battery_percent}%"`
    #[serde(default = "default_format_connected_battery")]
    pub connected_battery: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PopupConfig {
    /// Whether to make the popup scrollable or stretchable to show all of its content.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    pub scrollable: bool,

    /// Format string to use for the header of popup window.
    ///
    /// **Default**: `" Enable Bluetooth"`
    #[serde(default = "default_popup_header")]
    pub header: String,

    /// Format string to use for the message that is displayed when the adapter is not found or disabled.
    ///
    /// **Default**: `"{adapter_status}"`
    #[serde(default = "default_popup_disabled")]
    pub disabled: String,

    /// Device box related configuration
    #[serde(default)]
    pub device: PopupDeviceConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdapterStatus {
    /// The value of `{adapter_status}` formatting token when adapter is enabling.
    ///
    /// **Default**: `"Enabling Bluetooth..."`
    #[serde(default = "default_adapter_status_enabling")]
    pub enabling: String,

    /// The value of `{adapter_status}` formatting token when adapter is enabled.
    ///
    /// **Default**: `"Bluetooth enabled"`
    #[serde(default = "default_adapter_status_enabled")]
    pub enabled: String,

    /// The value of `{adapter_status}` formatting token when adapter is disabling.
    ///
    /// **Default**: `"Disabling Bluetooth..."`
    #[serde(default = "default_adapter_status_disabling")]
    pub disabling: String,

    /// The value of `{adapter_status}` formatting token when adapter is disabled.
    ///
    /// **Default**: `"Bluetooth disabled"`
    #[serde(default = "default_adapter_status_disabled")]
    pub disabled: String,

    /// The value of `{adapter_status}` formatting token when adapter not found.
    ///
    /// **Default**: `"No Bluetooth adapters found"`
    #[serde(default = "default_adapter_status_not_found")]
    pub not_found: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeviceStatus {
    /// The value of `{device_status}` formatting token when device is connecting.
    ///
    /// **Default**: `"Connecting..."`
    #[serde(default = "default_device_status_connecting")]
    pub connecting: String,

    /// The value of `{device_status}` formatting token when device is connected.
    ///
    /// **Default**: `"Connected"`
    #[serde(default = "default_device_status_connected")]
    pub connected: String,

    /// The value of `{device_status}` formatting token when device is disconnecting.
    ///
    /// **Default**: `"Disconnecting..."`
    #[serde(default = "default_device_status_disconnecting")]
    pub disconnecting: String,

    /// The value of `{device_status}` formatting token when device is disconnected.
    ///
    /// **Default**: `"Disconnect"`
    #[serde(default = "default_device_status_disconnected")]
    pub disconnected: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PopupDeviceConfig {
    /// Format string to use for the header of device box.
    ///
    /// **Default**: `"{device_alias}"`
    #[serde(default = "default_device_header")]
    pub header: String,

    /// Format string to use for the header of device box when `{device_battery_percent}` is available.
    ///
    /// **Default**: `"{device_alias}"`
    #[serde(default = "default_device_header")]
    pub header_battery: String,

    /// Format string to use for the footer of device box.
    ///
    /// **Default**: `"{device_status}"`
    #[serde(default = "default_device_footer")]
    pub footer: String,

    /// Format string to use for the footer of device box when `{device_battery_percent}` is available.
    ///
    /// **Default**: `"{device_status} • Battery {device_battery_percent}%"`
    #[serde(default = "default_device_footer_battery")]
    pub footer_battery: String,
}

const fn default_icon_size() -> i32 {
    32
}

fn default_format_not_found() -> String {
    "".into()
}

fn default_format_disabled() -> String {
    " Off".into()
}

fn default_format_enabled() -> String {
    " On".into()
}

fn default_format_connected() -> String {
    " {device_alias}".into()
}

fn default_format_connected_battery() -> String {
    " {device_alias} • {device_battery_percent}%".into()
}

fn default_popup_header() -> String {
    " Enable Bluetooth".into()
}

fn default_popup_disabled() -> String {
    "{adapter_status}".into()
}

fn default_device_header() -> String {
    "{device_alias}".into()
}

fn default_device_footer() -> String {
    "{device_status}".into()
}

fn default_device_footer_battery() -> String {
    "{device_status} • Battery {device_battery_percent}%".into()
}

fn default_adapter_status_enabling() -> String {
    "Enabling Bluetooth...".into()
}
fn default_adapter_status_enabled() -> String {
    "Bluetooth enabled".into()
}
fn default_adapter_status_disabling() -> String {
    "Disabling Bluetooth...".into()
}
fn default_adapter_status_disabled() -> String {
    "Bluetooth disabled".into()
}
fn default_adapter_status_not_found() -> String {
    "No Bluetooth adapters found".into()
}
fn default_device_status_connecting() -> String {
    "Connecting...".into()
}
fn default_device_status_connected() -> String {
    "Connected".into()
}
fn default_device_status_disconnecting() -> String {
    "Disconnecting...".into()
}
fn default_device_status_disconnected() -> String {
    "Disconnected".into()
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            not_found: default_format_not_found(),
            disabled: default_format_disabled(),
            enabled: default_format_enabled(),
            connected: default_format_connected(),
            connected_battery: default_format_connected_battery(),
        }
    }
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            scrollable: true,
            header: default_popup_header(),
            disabled: default_popup_disabled(),
            device: Default::default(),
        }
    }
}

impl Default for AdapterStatus {
    fn default() -> Self {
        Self {
            enabling: default_adapter_status_enabling(),
            enabled: default_adapter_status_enabled(),
            disabling: default_adapter_status_disabling(),
            disabled: default_adapter_status_disabled(),
            not_found: default_adapter_status_not_found(),
        }
    }
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            connecting: default_device_status_connecting(),
            connected: default_device_status_connected(),
            disconnecting: default_device_status_disconnecting(),
            disconnected: default_device_status_disconnected(),
        }
    }
}

impl Default for PopupDeviceConfig {
    fn default() -> Self {
        Self {
            header: default_device_header(),
            header_battery: default_device_header(),
            footer: default_device_footer(),
            footer_battery: default_device_footer_battery(),
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
            format!("{}", percent)
        } else {
            "".into()
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
