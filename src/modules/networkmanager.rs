use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::networkmanager::state::DeviceTypeData;
use crate::clients::networkmanager::{Client, DeviceState, DeviceType, NetworkManagerUpdate};
use crate::config::{CommonConfig, default};
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};

use color_eyre::Result;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::{Box as GtkBox, ContentFit, Picture};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

mod config;

/// A simplified version of [`DeviceState`] used for icon selection.
enum State {
    Disconnected,
    Acquiring,
    Connected,
}

impl From<DeviceState> for State {
    fn from(state: DeviceState) -> Self {
        match state {
            DeviceState::Unknown
            | DeviceState::Unmanaged
            | DeviceState::Unavailable
            | DeviceState::Deactivating
            | DeviceState::Failed
            | DeviceState::Disconnected => State::Disconnected,
            DeviceState::Prepare
            | DeviceState::Config
            | DeviceState::NeedAuth
            | DeviceState::IpConfig
            | DeviceState::IpCheck
            | DeviceState::Secondaries => State::Acquiring,
            DeviceState::Activated => State::Connected,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct NetworkManagerModule {
    /// The size of the icon for each network device, in pixels.
    icon_size: i32,

    /// The configuraiton for the icons used to represent network devices.
    #[serde(default)]
    icons: config::Icons,

    /// Any device with a type in this list will not be shown. The type is a string matching
    /// [`DeviceType`] variants (e.g. `"Wifi"`, `"Ethernet", etc.).
    #[serde(default)]
    types_blacklist: Vec<DeviceType>,

    /// If not empty, only devices with a type in this list will be shown. The type is a string
    /// matching [`DeviceType`] variants (e.g. `"Wifi"`, `"Ethernet", etc.).
    #[serde(default)]
    types_whitelist: Vec<DeviceType>,

    /// Any device whose interface name is in this list will not be shown.
    #[serde(default)]
    interface_blacklist: Vec<String>,

    /// If not empty, only devices whose interface name is in this list will be shown.
    #[serde(default)]
    interface_whitelist: Vec<String>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}
impl NetworkManagerModule {
    async fn update_icon(
        &self,
        image_provider: &crate::image::Provider,
        device: &crate::clients::networkmanager::state::Device,
        icon: &Picture,
    ) {
        fn whitelisted<T: PartialEq>(list: &[T], x: &T) -> bool {
            list.is_empty() || list.contains(x)
        }

        let type_whitelisted = whitelisted(&self.types_whitelist, &device.device_type);
        let interface_whitelisted = whitelisted(&self.interface_whitelist, &device.interface);
        let type_blacklisted = self.types_blacklist.contains(&device.device_type);
        let interface_blacklisted = self.interface_blacklist.contains(&device.interface);

        if !type_whitelisted || !interface_whitelisted || type_blacklisted || interface_blacklisted
        {
            icon.set_visible(false);
            return;
        }

        let mut tooltip = device.interface.clone();
        if let Some(ip) = &device.ip4_config {
            for x in &ip.address_data {
                tooltip.push('\n');
                tooltip.push_str(&x.address);
                tooltip.push('/');
                tooltip.push_str(&x.prefix.to_string());
            }
        }

        let state = State::from(device.state);

        let icon_name = match device.device_type {
            DeviceType::Wifi => match state {
                State::Acquiring => self.icons.wifi.acquiring.as_str(),
                State::Disconnected => self.icons.wifi.disconnected.as_str(),
                State::Connected => match &device.device_type_data {
                    DeviceTypeData::Wireless(wireless) => match &wireless.active_access_point {
                        Some(connection) => {
                            tooltip.push('\n');
                            tooltip.push_str(&String::from_utf8_lossy(&connection.ssid));

                            if self.icons.wifi.levels.is_empty() {
                                ""
                            } else {
                                let level = strength_to_level(
                                    connection.strength,
                                    self.icons.wifi.levels.len(),
                                );
                                self.icons.wifi.levels[level].as_str()
                            }
                        }
                        None => self.icons.wifi.disconnected.as_str(),
                    },
                    _ => self.icons.unknown.as_str(),
                },
            },
            DeviceType::Modem | DeviceType::Wimax => match state {
                State::Acquiring => self.icons.cellular.acquiring.as_ref(),
                State::Disconnected => self.icons.cellular.disconnected.as_ref(),
                State::Connected => self.icons.cellular.connected.as_ref(),
            },
            DeviceType::Wireguard
            | DeviceType::Tun
            | DeviceType::IpTunnel
            | DeviceType::Vxlan
            | DeviceType::Macsec => match state {
                State::Acquiring => self.icons.vpn.acquiring.as_ref(),
                State::Disconnected => self.icons.vpn.disconnected.as_ref(),
                State::Connected => self.icons.vpn.connected.as_ref(),
            },
            _ => match state {
                State::Acquiring => self.icons.wired.acquiring.as_ref(),
                State::Disconnected => self.icons.wired.disconnected.as_ref(),
                State::Connected => self.icons.wired.connected.as_ref(),
            },
        };

        if icon_name.is_empty() {
            icon.set_visible(false);
            return;
        }

        image_provider
            .load_into_picture_silent(icon_name, self.icon_size, false, icon)
            .await;
        icon.set_tooltip_text(Some(&tooltip));

        icon.set_visible(true);
    }
}

impl Default for NetworkManagerModule {
    fn default() -> Self {
        Self {
            icon_size: default::IconSize::Small as i32,
            common: Some(CommonConfig::default()),
            icons: config::Icons::default(),
            types_blacklist: Vec::new(),
            types_whitelist: Vec::new(),
            interface_blacklist: Vec::new(),
            interface_whitelist: Vec::new(),
        }
    }
}

impl Module<GtkBox> for NetworkManagerModule {
    type SendMessage = NetworkManagerUpdate;
    type ReceiveMessage = ();

    module_impl!("networkmanager");

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

        let container_clone = container.clone();
        context.subscribe().recv_glib_async((), move |(), update| {
            let container = container.clone();
            let image_provider = image_provider.clone();
            let this = self.clone();
            async move {
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
                            this.update_icon(
                                &image_provider,
                                device,
                                widget.downcast_ref::<Picture>().expect("should be Picture"),
                            )
                            .await;
                        }
                    }
                    NetworkManagerUpdate::Device(idx, device) => {
                        tracing::debug!(
                            "NetworkManager device {idx} updated: {}",
                            device.interface
                        );
                        tracing::trace!("NetworkManager device {idx} updated: {device:#?}");
                        if let Some(widget) = container.children().nth(idx) {
                            this.update_icon(
                                &image_provider,
                                &device,
                                widget.downcast_ref::<Picture>().expect("should be Picture"),
                            )
                            .await;
                        } else {
                            tracing::warn!("No widget found for device index {idx}");
                        }
                    }
                }
            }
        });

        Ok(ModuleParts::new(container_clone, None))
    }
}

/// Convert strength level (from 0-100), to a level (from 0 to `number_of_levels-1`).
fn strength_to_level(strength: u8, levels: usize) -> usize {
    // Strength levels based for the one show by [`nmcli dev wifi list`](https://github.com/NetworkManager/NetworkManager/blob/83a259597000a88217f3ccbdfe71c8114242e7a6/src/libnmc-base/nm-client-utils.c#L700-L727):
    // match strength {
    //     0..=4 => 0,
    //     5..=29 => 1,
    //     30..=54 => 2,
    //     55..=79 => 3,
    //     80.. => 4,
    // }

    // to make it work with a custom number of levels, we approach the logic above with a
    // piece-wise linear interpolation:
    // - 0 to 5 -> 0 to 0.2
    // - 5 to 80 -> 0.2 to 0.8
    // - 80 to 100 -> 0.8 to 1.0

    if levels <= 1 {
        return 0;
    }

    let strength = strength.clamp(0, 100);

    let pos = if strength < 5 {
        // Linear interpolation between 0..5
        (strength as f32 / 5.0) * 0.2
    } else if strength < 80 {
        // Linear interpolation between 5..80
        0.2 + ((strength - 5) as f32 / 75.0) * 0.6
    } else {
        // Linear interpolation between 80..100
        0.8 + ((strength as f32 - 80.0) / 20.0) * 0.2
    };

    // Scale to discrete levels
    let level = (pos * levels as f32).floor() as usize;
    level.min(levels - 1)
}

// Just to make sure the implementation still follow the reference logic
#[cfg(test)]
#[test]
fn test_strength_to_level() {
    for levels in 0..=10 {
        println!("Levels: {}", levels);
        for strength in (0..=100).step_by(5) {
            let level = strength_to_level(strength, levels);
            println!("  Strength: {:3} => Level: {}", strength, level);
        }
    }
    assert_eq!(strength_to_level(0, 5), 0);
    assert_eq!(strength_to_level(4, 5), 0);
    assert_eq!(strength_to_level(5, 5), 1);
    assert_eq!(strength_to_level(6, 5), 1);
    assert_eq!(strength_to_level(29, 5), 1);
    assert_eq!(strength_to_level(30, 5), 2);
    assert_eq!(strength_to_level(54, 5), 2);
    assert_eq!(strength_to_level(55, 5), 3);
    assert_eq!(strength_to_level(79, 5), 3);
    assert_eq!(strength_to_level(80, 5), 4);
    assert_eq!(strength_to_level(100, 5), 4);
}
