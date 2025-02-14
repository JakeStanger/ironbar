use std::collections::HashMap;
use std::ops::Deref;

use color_eyre::Result;
use glib::SignalHandlerId;
use gtk::{prelude::*, IconTheme, ScrolledWindow, Spinner};
use gtk::{Align, Button, Label, Orientation};
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
    /// Whether to show scroll lock indicator.
    ///
    ///  **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    scrollable: bool,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    #[serde(default)]
    adapter: AdapterStrings,

    #[serde(default)]
    adapter_status: AdapterStatus,

    #[serde(default)]
    device: DeviceStrings,

    #[serde(default)]
    device_status: DeviceStatus,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct AdapterStatus {
    #[serde(default = "default_adapter_enabling")]
    enabling: String,

    #[serde(default = "default_adapter_enabled")]
    enabled: String,

    #[serde(default = "default_adapter_disabling")]
    disabling: String,

    #[serde(default = "default_adapter_disabled")]
    disabled: String,

    #[serde(default = "default_adapter_not_found")]
    not_found: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct DeviceStatus {
    #[serde(default = "default_device_connecting")]
    connecting: String,

    #[serde(default = "default_device_connected")]
    connected: String,

    #[serde(default = "default_device_disconnecting")]
    disconnecting: String,

    #[serde(default = "default_device_disconnected")]
    disconnected: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct DeviceStrings {
    #[serde(default = "default_device_header")]
    device_header: String,

    #[serde(default = "default_device_header")]
    device_header_connected_battery: String,

    #[serde(default = "default_device_footer")]
    device_footer: String,

    #[serde(default = "default_device_footer_connected_battery")]
    device_footer_connected_battery: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct AdapterStrings {
    #[serde(default = "default_adapter_header")]
    header: String,

    #[serde(default = "default_adapter_status")]
    status: String,
}

const fn default_icon_size() -> i32 {
    32
}

fn default_adapter_header() -> String {
    " Enable Bluetooth".into()
}

fn default_adapter_status() -> String {
    "{adapter_status}".into()
}

fn default_device_header() -> String {
    "{device_alias}".into()
}

fn default_device_footer() -> String {
    "{device_status}".into()
}

fn default_device_footer_connected_battery() -> String {
    "{device_status} • Battery {device_battery_percent}%".into()
}

fn default_adapter_enabling() -> String {
    "Enabling...".into()
}
fn default_adapter_enabled() -> String {
    "Enabled".into()
}
fn default_adapter_disabling() -> String {
    "Disabling...".into()
}
fn default_adapter_disabled() -> String {
    "Bluetooth disabled".into()
}
fn default_adapter_not_found() -> String {
    "No Bluetooth adapters found".into()
}
fn default_device_connecting() -> String {
    "Connecting...".into()
}
fn default_device_connected() -> String {
    "Connected".into()
}
fn default_device_disconnecting() -> String {
    "Disconnecting...".into()
}
fn default_device_disconnected() -> String {
    "Disconnected".into()
}

impl Default for AdapterStatus {
    fn default() -> Self {
        Self {
            enabling: default_adapter_enabling(),
            enabled: default_adapter_enabled(),
            disabling: default_adapter_disabling(),
            disabled: default_adapter_disabled(),
            not_found: default_adapter_not_found(),
        }
    }
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            connecting: default_device_connecting(),
            connected: default_device_connected(),
            disconnecting: default_device_disconnecting(),
            disconnected: default_device_disconnected(),
        }
    }
}

impl Default for AdapterStrings {
    fn default() -> Self {
        Self {
            header: default_adapter_header(),
            status: default_adapter_status(),
        }
    }
}

impl Default for DeviceStrings {
    fn default() -> Self {
        Self {
            device_header: default_device_header(),
            device_header_connected_battery: default_device_header(),
            device_footer: default_device_footer(),
            device_footer_connected_battery: default_device_footer_connected_battery(),
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

        str.replace("{adapter_status}", &status)
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
            .replace("{device_status}", &status)
            .replace("{device_alias}", &device.alias)
            .replace("{device_battery_percent}", &battery_percent)
    }

    fn replace_adapter(
        state: &BluetoothState,
        adapter_status: &AdapterStatus,
        adapter: &AdapterStrings,
    ) -> AdapterStrings {
        AdapterStrings {
            header: Self::format_adapter(state, adapter_status, &adapter.header),
            status: Self::format_adapter(state, adapter_status, &adapter.status),
        }
    }

    fn replace_device(
        state: &BluetoothDevice,
        device_status: &DeviceStatus,
        device: &DeviceStrings,
    ) -> DeviceStrings {
        DeviceStrings {
            device_header: Self::format_device(state, device_status, &device.device_header),
            device_header_connected_battery: Self::format_device(
                state,
                device_status,
                &device.device_header_connected_battery,
            ),
            device_footer: Self::format_device(state, device_status, &device.device_footer),
            device_footer_connected_battery: Self::format_device(
                state,
                device_status,
                &device.device_footer_connected_battery,
            ),
        }
    }
}

impl BluetoothDeviceBox {
    fn new(tx: mpsc::Sender<BluetoothAction>, icon_theme: &IconTheme, icon_size: i32) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);
        container.add_class("device");
        let icon_box = IconLabel::new("", &icon_theme, icon_size);
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

    fn set_data(&mut self, data: BluetoothDevice, device_strings: &DeviceStrings) {
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

        if let Some(_) = data.battery_percent {
            self.header
                .set_text(&device_strings.device_header_connected_battery);
            self.footer
                .set_text(&device_strings.device_footer_connected_battery);
        } else {
            self.header.set_text(&device_strings.device_header);
            self.footer.set_text(&device_strings.device_footer);
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
        let label = Label::new(Some("test"));
        button.add(&label);

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            try_send!(tx, ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let rx = context.subscribe();
        glib_recv!(rx, _state => {
            let date_string = format!("{}", "Bluetooth");
            label.set_label(&date_string);
        });

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
            if self.scrollable {
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

            let device_strings = self.device;
            let adapter_strings = self.adapter;
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

                let adapter_strings =
                    Self::replace_adapter(&state, &adapter_status, &adapter_strings);

                header_label.set_text(&adapter_strings.header);
                status_label.set_text(&adapter_strings.status);

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
                        let (device_box, ref mut local_seq) =
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

                    // Remove devices without updates `seq` (i.e. not in `devices`)
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
