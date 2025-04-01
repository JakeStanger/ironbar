use std::collections::HashMap;
use std::ops::Deref;

use color_eyre::Result;
use glib::SignalHandlerId;
use gtk::{Align, Button, Label, Orientation};
use gtk::{IconTheme, ScrolledWindow, Spinner, prelude::*};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};

use crate::clients::bluetooth::{self, BluetoothDevice, BluetoothDeviceStatus, BluetoothState};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::IconLabel;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{glib_recv, module_impl, send_async, spawn, try_send};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BluetoothModule {
    /// Format strings for on-bar button
    #[serde(default)]
    format: FormatConfig,

    /// Popup related configuration
    #[serde(default)]
    popup: PopupConfig,

    /// Values of `{adapter_status}` formatting token.
    #[serde(default)]
    adapter_status: AdapterStatus,

    /// Values of `{device_status}` formatting token.
    #[serde(default)]
    device_status: DeviceStatus,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct FormatConfig {
    /// Format string to use for the widget button when bluetooth adapter not found.
    ///
    /// **Default**: `""`
    #[serde(default = "default_format_not_found")]
    not_found: String,

    /// Format string to use for the widget button when bluetooth adapter is disabled.
    ///
    /// **Default**: `" Off"`
    #[serde(default = "default_format_disabled")]
    disabled: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled but no devices are connected.
    ///
    /// **Default**: `" On"`
    #[serde(default = "default_format_enabled")]
    enabled: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled and a device is connected.
    ///
    /// **Default**: `" {device_alias}"`
    #[serde(default = "default_format_connected")]
    connected: String,

    /// Format string to use for the widget button when bluetooth adapter is enabled, a device is connected and `{device_battery_percent}` is available.
    ///
    /// **Default**: `" {device_alias} • {device_battery_percent}%"`
    #[serde(default = "default_format_connected_battery")]
    connected_battery: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct PopupConfig {
    /// Whether to make the popup scrollable or stretchable to show all of its content.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    scrollable: bool,

    /// Format string to use for the header of popup window.
    ///
    /// **Default**: `" Enable Bluetooth"`
    #[serde(default = "default_popup_header")]
    header: String,

    /// Format string to use for the status string that is displayed when the adapter is not found or disabled.
    ///
    /// **Default**: `"{adapter_status}"`
    #[serde(default = "default_popup_status")]
    status: String,

    /// Device box related configuration
    #[serde(default)]
    device: PopupDeviceConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct AdapterStatus {
    /// The value of `{adapter_status}` formatting token when adapter is enabling.
    ///
    /// **Default**: `"Enabling Bluetooth..."`
    #[serde(default = "default_adapter_status_enabling")]
    enabling: String,

    /// The value of `{adapter_status}` formatting token when adapter is enabled.
    ///
    /// **Default**: `"Bluetooth enabled"`
    #[serde(default = "default_adapter_status_enabled")]
    enabled: String,

    /// The value of `{adapter_status}` formatting token when adapter is disabling.
    ///
    /// **Default**: `"Disabling Bluetooth..."`
    #[serde(default = "default_adapter_status_disabling")]
    disabling: String,

    /// The value of `{adapter_status}` formatting token when adapter is disabled.
    ///
    /// **Default**: `"Bluetooth disabled"`
    #[serde(default = "default_adapter_status_disabled")]
    disabled: String,

    /// The value of `{adapter_status}` formatting token when adapter not found.
    ///
    /// **Default**: `"No Bluetooth adapters found"`
    #[serde(default = "default_adapter_status_not_found")]
    not_found: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct DeviceStatus {
    /// The value of `{device_status}` formatting token when device is connecting.
    ///
    /// **Default**: `"Connecting..."`
    #[serde(default = "default_device_status_connecting")]
    connecting: String,

    /// The value of `{device_status}` formatting token when device is connected.
    ///
    /// **Default**: `"Connected"`
    #[serde(default = "default_device_status_connected")]
    connected: String,

    /// The value of `{device_status}` formatting token when device is disconnecting.
    ///
    /// **Default**: `"Disconnecting..."`
    #[serde(default = "default_device_status_disconnecting")]
    disconnecting: String,

    /// The value of `{device_status}` formatting token when device is disconnected.
    ///
    /// **Default**: `"Disconnect"`
    #[serde(default = "default_device_status_disconnected")]
    disconnected: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct PopupDeviceConfig {
    /// Format string to use for the header of device box.
    ///
    /// **Default**: `"{device_alias}"`
    #[serde(default = "default_device_header")]
    header: String,

    /// Format string to use for the header of device box when `{device_battery_percent}` is available.
    ///
    /// **Default**: `"{device_alias}"`
    #[serde(default = "default_device_header")]
    header_battery: String,

    /// Format string to use for the footer of device box.
    ///
    /// **Default**: `"{device_status}"`
    #[serde(default = "default_device_footer")]
    footer: String,

    /// Format string to use for the footer of device box when `{device_battery_percent}` is available.
    ///
    /// **Default**: `"{device_status} • Battery {device_battery_percent}%"`
    #[serde(default = "default_device_footer_battery")]
    footer_battery: String,
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

fn default_popup_status() -> String {
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
            status: default_popup_status(),
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

pub enum BluetoothAction {
    Enable,
    Disable,
    Connect(bluer::Address),
    Disconnect(bluer::Address),
}

struct BluetoothDeviceBox {
    container: gtk::Box,
    icon_box: IconLabel,
    header: Label,

    footer: Label,
    spinner: Spinner,
    switch: gtk::Switch,
    switch_handler: Option<SignalHandlerId>,
    tx: mpsc::Sender<BluetoothAction>,
}

impl BluetoothModule {
    fn format_adapter(state: &BluetoothState, adapter_status: &AdapterStatus, str: &str) -> String {
        let status = match state {
            BluetoothState::Enabling => &adapter_status.enabling,
            BluetoothState::Enabled { .. } => &adapter_status.enabled,
            BluetoothState::Disabling => &adapter_status.disabling,
            BluetoothState::Disabled => &adapter_status.disabled,
            BluetoothState::NotFound => &adapter_status.not_found,
        };

        str.replace("{adapter_status}", status)
    }

    fn format_device(device: &BluetoothDevice, device_status: &DeviceStatus, str: &str) -> String {
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

    fn replace_device(
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

impl BluetoothDeviceBox {
    fn new(tx: mpsc::Sender<BluetoothAction>, icon_theme: &IconTheme, icon_size: i32) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);
        container.add_class("device");
        let icon_box = IconLabel::new("", icon_theme, icon_size);
        icon_box.add_class("icon-box");

        let status = gtk::Box::new(Orientation::Vertical, 0);
        status.add_class("status");
        status.set_valign(Align::Center);

        let header = Label::new(None);
        header.add_class("header-label");

        let footer = Label::new(None);
        footer.add_class("footer-label");

        let switch = gtk::Switch::new();
        switch.set_valign(Align::Center);
        switch.add_class("switch");

        let spinner = Spinner::new();
        spinner.add_class("spinner");
        spinner.start();

        icon_box.set_halign(Align::Start);
        header.set_halign(Align::Start);
        footer.set_halign(Align::Start);
        status.set_halign(Align::Start);

        spinner.set_halign(Align::End);
        switch.set_halign(Align::End);

        header.set_hexpand(true);

        container.add(&*icon_box);
        status.add(&header);
        status.add(&footer);
        container.add(&status);
        container.add(&spinner);
        container.add(&switch);
        container.show_all();

        Self {
            container,
            icon_box,
            header,
            footer,
            spinner,
            switch,
            switch_handler: None,
            tx,
        }
    }

    fn set_data(&mut self, data: BluetoothDevice, device_strings: &PopupDeviceConfig) {
        if let Some(icon) = data.icon {
            self.icon_box.set_label(Some(&format!("icon:{}", icon)));
        }
        self.header.set_text(&data.alias);

        if let Some(handler) = self.switch_handler.take() {
            self.switch.disconnect(handler);
        }

        self.spinner.set_active(matches!(
            data.status,
            BluetoothDeviceStatus::Connecting | BluetoothDeviceStatus::Disconnecting
        ));

        self.switch.set_sensitive(matches!(
            data.status,
            BluetoothDeviceStatus::Connected | BluetoothDeviceStatus::Disconnected
        ));
        self.switch.set_state(matches!(
            data.status,
            BluetoothDeviceStatus::Connected | BluetoothDeviceStatus::Connecting
        ));

        if data.battery_percent.is_some() {
            self.header.set_text(&device_strings.header_battery);
            self.footer.set_text(&device_strings.footer_battery);
        } else {
            self.header.set_text(&device_strings.header);
            self.footer.set_text(&device_strings.footer);
        }

        if matches!(data.status, BluetoothDeviceStatus::Disconnected) {
            let tx = self.tx.clone();
            self.switch_handler = Some(self.switch.connect_active_notify(move |switch| {
                try_send!(tx, BluetoothAction::Connect(data.address));
                switch.set_sensitive(false);
            }));
        }

        if matches!(data.status, BluetoothDeviceStatus::Connected) {
            let tx = self.tx.clone();
            self.switch_handler = Some(self.switch.connect_active_notify(move |switch| {
                try_send!(tx, BluetoothAction::Disconnect(data.address));
                switch.set_sensitive(false);
            }));
        }
    }
}

impl Deref for BluetoothDeviceBox {
    type Target = gtk::Box;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl Module<Button> for BluetoothModule {
    type SendMessage = BluetoothState;
    type ReceiveMessage = BluetoothAction;

    module_impl!("bluetooth");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let client = context.try_client::<bluetooth::Client>()?;

        {
            let tx = context.tx.clone();
            let mut rx = client.subscribe();
            spawn(async move {
                loop {
                    let state = rx.borrow_and_update().clone();
                    send_async!(tx, ModuleUpdateEvent::Update(state));

                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            });
        }

        // ui events
        spawn(async move {
            while let Some(action) = rx.recv().await {
                match action {
                    BluetoothAction::Enable => client.enable(),
                    BluetoothAction::Disable => client.disable(),
                    BluetoothAction::Connect(addr) => client.connect(addr),
                    BluetoothAction::Disconnect(addr) => client.disconnect(addr),
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<Button>> {
        let button = Button::new();
        let label = Label::new(None);
        button.add(&label);

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            try_send!(tx, ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        {
            let rx = context.subscribe();
            let format_strings = self.format.clone();
            let device_status = self.device_status.clone();
            let adapter_status = self.adapter_status.clone();
            let button = button.clone();

            let handle_state = move |state: BluetoothState| {
                let (text, class) = match &state {
                    BluetoothState::NotFound => (
                        Self::format_adapter(&state, &adapter_status, &format_strings.not_found),
                        "not-found",
                    ),
                    BluetoothState::Disabled
                    | BluetoothState::Enabling
                    | BluetoothState::Disabling => (
                        Self::format_adapter(&state, &adapter_status, &format_strings.disabled),
                        "disabled",
                    ),
                    BluetoothState::Enabled { devices } => match devices.iter().next() {
                        Some(device)
                            if matches!(device.status, BluetoothDeviceStatus::Connected) =>
                        {
                            let res = if device.battery_percent.is_some() {
                                let res = Self::format_adapter(
                                    &state,
                                    &adapter_status,
                                    &format_strings.connected_battery,
                                );
                                Self::format_device(device, &device_status, &res)
                            } else {
                                let res = Self::format_adapter(
                                    &state,
                                    &adapter_status,
                                    &format_strings.connected,
                                );
                                Self::format_device(device, &device_status, &res)
                            };
                            (res, "connected")
                        }
                        _ => (
                            Self::format_adapter(&state, &adapter_status, &format_strings.enabled),
                            "enabled",
                        ),
                    },
                };

                label.set_text(&text);

                button.remove_class("not-found");
                button.remove_class("disabled");
                button.remove_class("enabled");
                button.remove_class("connected");
                button.add_class(class);
            };

            glib_recv!(rx, handle_state);
        }

        let popup = self
            .into_popup(
                context.controller_tx.clone(),
                context.subscribe(),
                context,
                info,
            )
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let container = gtk::Box::new(Orientation::Vertical, 0);

        let header = gtk::Box::new(Orientation::Horizontal, 0);
        header.add_class("header");

        let header_switch = gtk::Switch::new();
        header_switch.add_class("switch");

        let header_label = Label::new(None);
        header_label.add_class("label");

        header.add(&header_switch);
        header.add(&header_label);

        container.add(&header);

        let devices = ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        devices.set_policy(
            gtk::PolicyType::Never,
            if self.popup.scrollable {
                gtk::PolicyType::Automatic
            } else {
                gtk::PolicyType::Never
            },
        );
        devices.set_vexpand(true);
        devices.add_class("devices");

        let devices_box = gtk::Box::new(Orientation::Vertical, 0);
        devices_box.add_class("box");

        devices.add(&devices_box);
        container.add(&devices);

        let status = gtk::Box::new(Orientation::Vertical, 0);
        status.add_class("status");
        status.set_valign(Align::Center);
        status.set_vexpand(true);

        let status_spinner = Spinner::new();
        status_spinner.add_class("spinner");
        status_spinner.start();
        status.add(&status_spinner);

        let status_label = Label::new(None);
        status_label.add_class("label");
        status.add(&status_label);

        container.add(&status);

        container.show_all();

        {
            let icon_size = self.icon_size;
            let icon_theme = info.icon_theme.clone();

            let popup_header = self.popup.header;
            let popup_status = self.popup.status;
            let device_strings = self.popup.device;
            let device_status = self.device_status;
            let adapter_status = self.adapter_status;

            let mut enable_handle = None;
            let mut device_map = HashMap::new();
            let mut seq = 0u32;
            let mut num_pinned = 0;

            let mut handle_state = move |state: BluetoothState| {
                if let Some(handle) = enable_handle.take() {
                    header_switch.disconnect(handle);
                }

                devices.set_visible(matches!(state, BluetoothState::Enabled { .. }));

                status.set_visible(!matches!(state, BluetoothState::Enabled { .. }));
                status_spinner.set_visible(matches!(
                    state,
                    BluetoothState::Enabling | BluetoothState::Disabling
                ));

                header_label.set_text(&Self::format_adapter(
                    &state,
                    &adapter_status,
                    &popup_header,
                ));
                status_label.set_text(&Self::format_adapter(
                    &state,
                    &adapter_status,
                    &popup_status,
                ));

                header.set_visible(!matches!(state, BluetoothState::NotFound));
                header_switch.set_sensitive(matches!(
                    state,
                    BluetoothState::Enabled { .. } | BluetoothState::Disabled
                ));

                {
                    enable_handle
                        .as_ref()
                        .inspect(|h| header_switch.block_signal(h));
                    header_switch.set_state(matches!(
                        state,
                        BluetoothState::Enabled { .. } | BluetoothState::Enabling
                    ));
                    enable_handle
                        .as_ref()
                        .inspect(|h| header_switch.unblock_signal(h));
                }

                if matches!(state, BluetoothState::Disabled) {
                    let tx = tx.clone();
                    enable_handle = Some(header_switch.connect_active_notify(move |switch| {
                        try_send!(tx, BluetoothAction::Enable);
                        switch.set_sensitive(false);
                    }));
                }

                if matches!(state, BluetoothState::Enabled { .. }) {
                    let tx = tx.clone();
                    enable_handle = Some(header_switch.connect_active_notify(move |switch| {
                        try_send!(tx, BluetoothAction::Disable);
                        switch.set_sensitive(false);
                    }));
                }

                if let BluetoothState::Enabled { devices } = state {
                    // `seq` is used here to find device boxes to remove
                    seq = seq.wrapping_add(1);

                    for device in devices {
                        let (device_box, local_seq) =
                            device_map.entry(device.address).or_insert_with(|| {
                                let device_box =
                                    BluetoothDeviceBox::new(tx.clone(), &icon_theme, icon_size);
                                devices_box.add(&*device_box);

                                (device_box, seq)
                            });

                        // Update seq of each devices to the latest one
                        *local_seq = seq;

                        // Pin non-disconnected devices to the top and unpin other types
                        let pos = devices_box.child_position(Deref::deref(&*device_box));
                        if matches!(device.status, BluetoothDeviceStatus::Disconnected) {
                            // Unpin
                            if pos < num_pinned {
                                num_pinned -= 1;

                                if pos != num_pinned {
                                    devices_box
                                        .reorder_child(Deref::deref(&*device_box), num_pinned);
                                }
                            }
                        } else {
                            // Pin
                            if pos >= num_pinned {
                                if pos != num_pinned {
                                    devices_box
                                        .reorder_child(Deref::deref(&*device_box), num_pinned);
                                }

                                num_pinned += 1;
                            }
                        }

                        let strings =
                            &Self::replace_device(&device, &device_status, &device_strings);
                        device_box.set_data(device, strings);
                    }

                    // Remove devices without updated `seq` (i.e. not in `devices`)
                    device_map.retain(|_, (device_box, local_seq)| {
                        if *local_seq != seq {
                            let pos = devices_box.child_position(Deref::deref(&*device_box));
                            if pos < num_pinned {
                                num_pinned -= 1;
                            }

                            devices_box.remove(Deref::deref(&*device_box));
                            false
                        } else {
                            true
                        }
                    });
                }
            };

            glib_recv!(rx, handle_state);
        }

        Some(container)
    }
}
