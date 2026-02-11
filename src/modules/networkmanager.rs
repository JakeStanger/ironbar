mod config;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::networkmanager::state::DeviceTypeData;
use crate::clients::networkmanager::{Client, DeviceType, NetworkManagerUpdate};
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::Provider;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};

use color_eyre::Result;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::{Box as GtkBox, ContentFit, Picture};
use tokio::sync::mpsc::Receiver;

pub use config::NetworkManagerModule;

use self::config::{ConnectionState, ProfileState, WifiConnectionState};

impl NetworkManagerModule {
    fn get_tooltip(&self, device: &crate::clients::networkmanager::state::Device) -> String {
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

    fn get_profile_state(
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

impl Module<GtkBox> for NetworkManagerModule {
    type SendMessage = NetworkManagerUpdate;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn on_create(&mut self) {
        self.profiles.setup_defaults(config::default_profiles());
    }

    fn spawn_controller(
        &self,
        _: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, ()>,
        _: Receiver<()>,
    ) -> Result<()> {
        let client = context.try_client::<Client>()?;
        let tx = context.tx.clone();

        spawn(async move {
            let mut client_signal = client.subscribe().await;
            while let Ok(state) = client_signal.recv().await {
                tx.send_update(state).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, ()>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<GtkBox>> {
        let container = GtkBox::new(info.bar_position.orientation(), 0);

        let image_provider = context.ironbar.image_provider();

        let icon_size = self.icon_size;
        let mut manager = self.profiles.attach(&container, move |_, event| {
            let (widget, image_provider): (gtk::Widget, Provider) = event.data;
            let icon_name = event.profile.icon.clone();
            tracing::debug!("profiles update: icon_name={icon_name}");
            if icon_name.is_empty() {
                widget.set_visible(false);
                return;
            }

            glib::spawn_future_local(async move {
                image_provider
                    .load_into_picture_silent(
                        &icon_name,
                        icon_size,
                        false,
                        widget.downcast_ref::<Picture>().expect("should be Picture"),
                    )
                    .await;
                widget.set_visible(true)
            });
        });

        let container_clone = container.clone();
        context.subscribe().recv_glib((), move |(), update| {
            match update {
                NetworkManagerUpdate::Devices(devices) => {
                    tracing::debug!("NetworkManager devices updated");
                    tracing::trace!("NetworkManager devices updated: {devices:#?}");

                    // resize the container's children to match the number of devices
                    if container.children().count() > devices.len() {
                        for child in container.children().skip(devices.len()) {
                            container.remove(&child);
                        }
                    } else {
                        while container.children().count() < devices.len() {
                            let icon = Picture::builder()
                                .content_fit(ContentFit::ScaleDown)
                                .css_classes(["icon"])
                                .build();
                            container.append(&icon);
                        }
                    }

                    // update each icon to match the device state
                    for (device, widget) in devices.iter().zip(container.children()) {
                        match self.get_profile_state(device) {
                            Some(state) => {
                                let tooltip = self.get_tooltip(device);
                                widget.set_tooltip_text(Some(&tooltip));
                                manager.update(state, (widget, image_provider.clone()));
                            }
                            _ => {
                                widget.set_visible(false);
                                continue;
                            }
                        };
                    }
                }
                NetworkManagerUpdate::Device(idx, device) => {
                    tracing::debug!("NetworkManager device {idx} updated: {}", device.interface);
                    tracing::trace!("NetworkManager device {idx} updated: {device:#?}");
                    if let Some(widget) = container.children().nth(idx) {
                        match self.get_profile_state(&device) {
                            Some(state) => {
                                let tooltip = self.get_tooltip(&device);
                                widget.set_tooltip_text(Some(&tooltip));
                                manager.update(state, (widget, image_provider.clone()));
                            }
                            _ => {
                                widget.set_visible(false);
                            }
                        };
                    } else {
                        tracing::warn!("No widget found for device index {idx}");
                    }
                }
            }
        });

        Ok(ModuleParts::new(container_clone, None))
    }
}
