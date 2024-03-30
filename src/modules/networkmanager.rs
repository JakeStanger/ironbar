use color_eyre::Result;
use futures_lite::StreamExt;
use gtk::prelude::*;
use gtk::{Image, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::zvariant::ObjectPath;

use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::ImageProvider;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, send_async, spawn};

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkmanagerModule {
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_icon_size() -> i32 {
    24
}

#[derive(Clone, Debug)]
pub enum NetworkmanagerState {
    Cellular,
    Offline,
    Unknown,
    Vpn,
    Wired,
    Wireless,
    WirelessDisconnected,
}

impl Module<gtk::Box> for NetworkmanagerModule {
    type SendMessage = NetworkmanagerState;
    type ReceiveMessage = ();

    fn name() -> &'static str {
        "networkmanager"
    }

    fn spawn_controller(
        &self,
        _: &ModuleInfo,
        context: &WidgetContext<NetworkmanagerState, ()>,
        _: Receiver<()>,
    ) -> Result<()> {
        let tx = context.tx.clone();

        spawn(async move {
            /* TODO: This should be moved into a client à la the upower module, however that
               requires additional refactoring as both would request a PropertyProxy but on
               different buses. The proper solution will be to rewrite both to use trait-derived
               proxies. */
            let nm_proxy = {
                let dbus = zbus::Connection::system().await?;
                PropertiesProxy::builder(&dbus)
                    .destination("org.freedesktop.NetworkManager")?
                    .path("/org/freedesktop/NetworkManager")?
                    .build()
                    .await?
            };
            let device_interface_name =
                InterfaceName::from_static_str("org.freedesktop.NetworkManager")?;

            let state = get_network_state(&nm_proxy, &device_interface_name).await?;
            send_async!(tx, ModuleUpdateEvent::Update(state));

            let mut prop_changed_stream = nm_proxy.receive_properties_changed().await?;
            while let Some(signal) = prop_changed_stream.next().await {
                let args = signal.args()?;
                if args.interface_name != device_interface_name {
                    continue;
                }

                let state = get_network_state(&nm_proxy, &device_interface_name).await?;
                send_async!(tx, ModuleUpdateEvent::Update(state));
            }

            Result::<()>::Ok(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<NetworkmanagerState, ()>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let container = gtk::Box::new(Orientation::Horizontal, 0);
        let icon = Image::new();
        icon.add_class("icon");
        container.add(&icon);

        let icon_theme = info.icon_theme.clone();

        let initial_icon_name = "icon:content-loading-symbolic";
        ImageProvider::parse(initial_icon_name, &icon_theme, false, self.icon_size)
            .map(|provider| provider.load_into_image(icon.clone()));

        let rx = context.subscribe();
        glib_recv!(rx, state => {
            let icon_name = match state {
                NetworkmanagerState::Cellular => "network-cellular-symbolic",
                NetworkmanagerState::Offline => "network-wireless-disabled-symbolic",
                NetworkmanagerState::Unknown => "dialog-question-symbolic",
                NetworkmanagerState::Vpn => "network-vpn-symbolic",
                NetworkmanagerState::Wired => "network-wired-symbolic",
                NetworkmanagerState::Wireless => "network-wireless-symbolic",
                NetworkmanagerState::WirelessDisconnected => "network-wireless-acquiring-symbolic",
            };
            ImageProvider::parse(icon_name, &icon_theme, false, self.icon_size)
                .map(|provider| provider.load_into_image(icon.clone()));
        });

        Ok(ModuleParts::new(container, None))
    }
}

async fn get_network_state(
    nm_proxy: &PropertiesProxy<'_>,
    device_interface_name: &InterfaceName<'_>,
) -> Result<NetworkmanagerState> {
    let properties = nm_proxy.get_all(device_interface_name.clone()).await?;

    let primary_connection_path = properties["PrimaryConnection"]
        .downcast_ref::<ObjectPath>()
        .unwrap();

    if primary_connection_path != "/" {
        let primary_connection_type = properties["PrimaryConnectionType"]
            .downcast_ref::<str>()
            .unwrap()
            .to_string();

        match primary_connection_type.as_str() {
            "802-11-olpc-mesh" => Ok(NetworkmanagerState::Wireless),
            "802-11-wireless" => Ok(NetworkmanagerState::Wireless),
            "802-3-ethernet" => Ok(NetworkmanagerState::Wired),
            "adsl" => Ok(NetworkmanagerState::Wired),
            "cdma" => Ok(NetworkmanagerState::Cellular),
            "gsm" => Ok(NetworkmanagerState::Cellular),
            "pppoe" => Ok(NetworkmanagerState::Wired),
            "vpn" => Ok(NetworkmanagerState::Vpn),
            "wifi-p2p" => Ok(NetworkmanagerState::Wireless),
            "wimax" => Ok(NetworkmanagerState::Cellular),
            "wireguard" => Ok(NetworkmanagerState::Vpn),
            "wpan" => Ok(NetworkmanagerState::Wireless),
            _ => Ok(NetworkmanagerState::Unknown),
        }
    } else {
        let wireless_enabled = *properties["WirelessEnabled"]
            .downcast_ref::<bool>()
            .unwrap();
        if wireless_enabled {
            Ok(NetworkmanagerState::WirelessDisconnected)
        } else {
            Ok(NetworkmanagerState::Offline)
        }
    }
}
