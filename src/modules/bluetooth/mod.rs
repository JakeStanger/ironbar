use std::collections::HashMap;
use std::ops::Deref;

use color_eyre::Result;
use glib::SignalHandlerId;
use gtk::{Align, Button, Label, Orientation};
use gtk::{ScrolledWindow, Spinner, prelude::*};
use tokio::sync::mpsc;

pub use self::config::BluetoothModule;
use self::config::PopupDeviceConfig;
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::bluetooth::{self, BluetoothDevice, BluetoothDeviceStatus, BluetoothState};
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::IconLabel;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{image, module_impl, spawn};

mod config;

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

impl BluetoothDeviceBox {
    fn new(
        tx: mpsc::Sender<BluetoothAction>,
        icon_size: i32,
        image_provider: &image::Provider,
    ) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);
        container.add_css_class("device");
        let icon_box = IconLabel::new("", icon_size, image_provider);
        icon_box.add_css_class("icon-box");

        let status = gtk::Box::new(Orientation::Vertical, 0);
        status.add_css_class("status");
        status.set_valign(Align::Center);

        let header = Label::new(None);
        header.add_css_class("header-label");

        let footer = Label::new(None);
        footer.add_css_class("footer-label");

        let switch = gtk::Switch::new();
        switch.set_valign(Align::Center);
        switch.add_css_class("switch");

        let spinner = Spinner::new();
        spinner.add_css_class("spinner");
        spinner.start();

        icon_box.set_halign(Align::Start);
        header.set_halign(Align::Start);
        footer.set_halign(Align::Start);
        status.set_halign(Align::Start);

        spinner.set_halign(Align::End);
        switch.set_halign(Align::End);

        header.set_hexpand(true);

        container.append(&*icon_box);
        status.append(&header);
        status.append(&footer);
        container.append(&status);
        container.append(&spinner);
        container.append(&switch);

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
            self.icon_box.set_label(Some(&format!("icon:{icon}")));
        }
        self.header.set_text(&data.alias);

        if let Some(handler) = self.switch_handler.take() {
            self.switch.disconnect(handler);
        }

        self.spinner.set_spinning(
            data.status == BluetoothDeviceStatus::Connecting
                || data.status == BluetoothDeviceStatus::Disconnecting,
        );

        self.switch.set_sensitive(
            data.status == BluetoothDeviceStatus::Connected
                || data.status == BluetoothDeviceStatus::Disconnected,
        );
        self.switch.set_state(
            data.status == BluetoothDeviceStatus::Connected
                || data.status == BluetoothDeviceStatus::Connecting,
        );

        if data.battery_percent.is_some() {
            self.header.set_text(&device_strings.header_battery);
            self.footer.set_text(&device_strings.footer_battery);
        } else {
            self.header.set_text(&device_strings.header);
            self.footer.set_text(&device_strings.footer);
        }

        if data.status == BluetoothDeviceStatus::Disconnected {
            let tx = self.tx.clone();
            self.switch_handler = Some(self.switch.connect_active_notify(move |switch| {
                tx.send_spawn(BluetoothAction::Connect(data.address));
                switch.set_sensitive(false);
            }));
        }

        if data.status == BluetoothDeviceStatus::Connected {
            let tx = self.tx.clone();
            self.switch_handler = Some(self.switch.connect_active_notify(move |switch| {
                tx.send_spawn(BluetoothAction::Disconnect(data.address));
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
                    tx.send_update(state).await;

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
        button.set_child(Some(&label));

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        {
            let rx = context.subscribe();
            let format_strings = self.format.clone();
            let device_status = self.device_status.clone();
            let adapter_status = self.adapter_status.clone();
            let button = button.clone();

            rx.recv_glib((), move |(), state: BluetoothState| {
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
                        Some(device) if device.status == BluetoothDeviceStatus::Connected => {
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

                button.remove_css_class("not-found");
                button.remove_css_class("disabled");
                button.remove_css_class("enabled");
                button.remove_css_class("connected");
                button.add_css_class(class);
            });
        }

        let popup = self
            .into_popup(context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let tx = context.controller_tx.clone();

        let container = gtk::Box::new(Orientation::Vertical, 0);

        let header = gtk::Box::new(Orientation::Horizontal, 0);
        header.add_css_class("header");

        let header_switch = gtk::Switch::new();
        header_switch.add_css_class("switch");

        let header_label = Label::new(None);
        header_label.add_css_class("label");

        header.append(&header_switch);
        header.append(&header_label);

        container.append(&header);

        let devices = ScrolledWindow::new();
        devices.set_policy(
            gtk::PolicyType::Never,
            if self.popup.scrollable {
                gtk::PolicyType::Automatic
            } else {
                gtk::PolicyType::Never
            },
        );
        devices.set_vexpand(true);
        devices.add_css_class("devices");

        let devices_box = gtk::Box::new(Orientation::Vertical, 0);
        devices_box.add_css_class("box");

        devices.set_child(Some(&devices_box));
        container.append(&devices);

        let disabled = gtk::Box::new(Orientation::Vertical, 0);
        disabled.add_css_class("disabled");
        disabled.set_valign(Align::Center);
        disabled.set_vexpand(true);

        let disabled_spinner = Spinner::new();
        disabled_spinner.add_css_class("spinner");
        disabled_spinner.start();
        disabled.append(&disabled_spinner);

        let disabled_label = Label::new(None);
        disabled_label.add_css_class("label");
        disabled.append(&disabled_label);

        container.append(&disabled);

        {
            let icon_size = self.icon_size;

            let popup_header = self.popup.header;
            let popup_grow_height_until = self.popup.grow_height_until;
            let popup_disabled = self.popup.disabled;
            let device_strings = self.popup.device;
            let device_status = self.device_status;
            let adapter_status = self.adapter_status;

            let mut enable_handle = None;
            let mut device_map = HashMap::new();
            let mut seq = 0u32;
            let mut num_pinned = 0;

            let rx = context.subscribe();
            rx.recv_glib((), move |(), state: BluetoothState| {
                if let Some(handle) = enable_handle.take() {
                    header_switch.disconnect(handle);
                }

                devices.set_visible(state.is_enabled());
                //ensure the content is at least some basic height to see at least 2 devices, everything else can be done per css: min-height
                devices.set_min_content_height(devices_box.height().min(popup_grow_height_until));

                disabled.set_visible(!state.is_enabled());
                disabled_spinner.set_visible(
                    state == BluetoothState::Enabling || state == BluetoothState::Disabling,
                );

                header_label.set_text(&Self::format_adapter(
                    &state,
                    &adapter_status,
                    &popup_header,
                ));
                disabled_label.set_text(&Self::format_adapter(
                    &state,
                    &adapter_status,
                    &popup_disabled,
                ));

                header.set_visible(state != BluetoothState::NotFound);
                header_switch
                    .set_sensitive(state.is_enabled() || state == BluetoothState::Disabled);

                {
                    enable_handle
                        .as_ref()
                        .inspect(|h| header_switch.block_signal(h));
                    header_switch
                        .set_state(state.is_enabled() || state == BluetoothState::Enabling);
                    enable_handle
                        .as_ref()
                        .inspect(|h| header_switch.unblock_signal(h));
                }

                if let BluetoothState::Enabled { devices } = state {
                    {
                        let tx = tx.clone();
                        enable_handle = Some(header_switch.connect_active_notify(move |switch| {
                            tx.send_spawn(BluetoothAction::Disable);
                            switch.set_sensitive(false);
                        }));
                    }

                    // `seq` is used here to find device boxes to remove
                    seq = seq.wrapping_add(1);

                    let image_provider = context.ironbar.image_provider();
                    for device in devices {
                        let (device_box, local_seq) =
                            device_map.entry(device.address).or_insert_with(|| {
                                let device_box =
                                    BluetoothDeviceBox::new(tx.clone(), icon_size, &image_provider);

                                devices_box.append(&*device_box);

                                (device_box, seq)
                            });

                        // Update seq of each devices to the latest one
                        *local_seq = seq;

                        // Pin non-disconnected devices to the top and unpin other types
                        let pos = devices_box
                            .children()
                            .position(|w| w == device_box.container)
                            .unwrap_or_default();

                        if device.status == BluetoothDeviceStatus::Disconnected {
                            // Unpin
                            if pos < num_pinned {
                                num_pinned -= 1;

                                if pos != num_pinned {
                                    let before = devices_box.children().nth(num_pinned);
                                    devices_box.reorder_child_after(
                                        &device_box.container,
                                        before.clone().as_ref(),
                                    );
                                }
                            }
                        } else if pos >= num_pinned {
                            // Pin
                            if pos != num_pinned {
                                let before = devices_box.children().nth(num_pinned);
                                devices_box.reorder_child_after(
                                    &device_box.container,
                                    before.clone().as_ref(),
                                );
                            }

                            num_pinned += 1;
                        }

                        let strings =
                            &Self::replace_device(&device, &device_status, &device_strings);
                        device_box.set_data(device, strings);
                    }

                    // Remove devices without updated `seq` (i.e. not in `devices`)
                    device_map.retain(|_, (device_box, local_seq)| {
                        if *local_seq == seq {
                            true
                        } else {
                            let pos = devices_box
                                .children()
                                .position(|w| w == device_box.container)
                                .unwrap_or_default();

                            if pos < num_pinned {
                                num_pinned -= 1;
                            }

                            devices_box.remove(&device_box.container);
                            false
                        }
                    });
                } else if state == BluetoothState::Disabled {
                    let tx = tx.clone();
                    enable_handle = Some(header_switch.connect_active_notify(move |switch| {
                        tx.send_spawn(BluetoothAction::Enable);
                        switch.set_sensitive(false);
                    }));
                }
            });
        }

        Some(container)
    }
}
