use std::collections::HashMap;

use color_eyre::Result;
use futures_lite::StreamExt;
use gtk::prelude::ContainerExt;
use gtk::{Image, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;
use zbus::dbus_proxy;
use zbus::names::InterfaceName;
use zbus::zvariant::{ObjectPath, Value};

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
    Wired,
    Wireless,
    WirelessDisconnected,
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkmanagerDBus {
    #[dbus_proxy(property)]
    fn active_connections(&self) -> Result<Vec<ObjectPath>>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<ObjectPath>>;

    #[dbus_proxy(property)]
    fn networking_enabled(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn primary_connection(&self) -> Result<ObjectPath>;

    #[dbus_proxy(property)]
    fn primary_connection_type(&self) -> Result<String>;

    #[dbus_proxy(property)]
    fn wireless_enabled(&self) -> Result<bool>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.DBus.Properties",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkmanagerPropsDBus {
    #[dbus_proxy(signal)]
    fn properties_changed(
        &self,
        interface_name: InterfaceName<'s>,
        changed_properties: HashMap<&'s str, Value<'s>>,
        invalidated_properties: Vec<&'s str>,
    ) -> Result<()>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
trait ActiveConnectionDBus {
    #[dbus_proxy(property)]
    fn connection(&self) -> Result<ObjectPath>;

    #[dbus_proxy(property)]
    fn default(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn default6(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<ObjectPath>>;

    #[dbus_proxy(property)]
    fn id(&self) -> Result<String>;

    #[dbus_proxy(property)]
    fn specific_object(&self) -> Result<ObjectPath>;

    #[dbus_proxy(property)]
    fn type_(&self) -> Result<String>;

    #[dbus_proxy(property)]
    fn vpn(&self) -> Result<bool>;
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
            // TODO: Maybe move this into a `client` à la `upower`?
            let dbus = zbus::Connection::system().await?;
            let nm_proxy = NetworkmanagerDBusProxy::new(&dbus).await?;
            let nm_props_proxy = NetworkmanagerPropsDBusProxy::new(&dbus).await?;

            let state = get_network_state(&nm_proxy).await?;
            send_async!(tx, ModuleUpdateEvent::Update(state));

            let mut prop_changed_stream = nm_props_proxy.receive_properties_changed().await?;
            while prop_changed_stream.next().await.is_some() {
                let state = get_network_state(&nm_proxy).await?;
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

async fn get_network_state(nm_proxy: &NetworkmanagerDBusProxy<'_>) -> Result<NetworkmanagerState> {
    let primary_connection_path = nm_proxy.primary_connection().await?;
    if primary_connection_path != "/" {
        let primary_connection_type = nm_proxy.primary_connection_type().await?;
        match primary_connection_type.as_str() {
            "802-11-olpc-mesh" => Ok(NetworkmanagerState::Wireless),
            "802-11-wireless" => Ok(NetworkmanagerState::Wireless),
            "802-3-ethernet" => Ok(NetworkmanagerState::Wired),
            "adsl" => Ok(NetworkmanagerState::Wired),
            "cdma" => Ok(NetworkmanagerState::Cellular),
            "gsm" => Ok(NetworkmanagerState::Cellular),
            "pppoe" => Ok(NetworkmanagerState::Wired),
            "wifi-p2p" => Ok(NetworkmanagerState::Wireless),
            "wimax" => Ok(NetworkmanagerState::Cellular),
            "wpan" => Ok(NetworkmanagerState::Wireless),
            _ => Ok(NetworkmanagerState::Unknown),
        }
    } else {
        let wireless_enabled = nm_proxy.wireless_enabled().await?;
        if wireless_enabled {
            Ok(NetworkmanagerState::WirelessDisconnected)
        } else {
            Ok(NetworkmanagerState::Offline)
        }
    }
}
