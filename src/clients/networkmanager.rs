use std::sync::Arc;

use crate::{register_fallible_client, spawn};
use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::error;
use zbus::export::ordered_stream::OrderedStreamExt;
use zbus::fdo::PropertiesProxy;
use zbus::{
    Connection, Result,
    names::InterfaceName,
    proxy,
    zvariant::{ObjectPath, Str},
};

const DBUS_BUS: &str = "org.freedesktop.NetworkManager";
const DBUS_PATH: &str = "/org/freedesktop/NetworkManager";
const DBUS_INTERFACE: &str = "org.freedesktop.NetworkManager";

#[derive(Debug)]
pub struct Client {
    client_state: Mutable<ClientState>,
    interface_name: InterfaceName<'static>,
    dbus_connection: Connection,
    props_proxy: PropertiesProxy<'static>,
}

#[derive(Clone, Debug)]
pub enum ClientState {
    WiredConnected,
    WifiConnected,
    CellularConnected,
    VpnConnected,
    WifiDisconnected,
    Offline,
    Unknown,
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManagerDbus {
    #[zbus(property)]
    fn active_connections(&self) -> Result<Vec<ObjectPath<'_>>>;

    #[zbus(property)]
    fn devices(&self) -> Result<Vec<ObjectPath<'_>>>;

    #[zbus(property)]
    fn networking_enabled(&self) -> Result<bool>;

    #[zbus(property)]
    fn primary_connection(&self) -> Result<ObjectPath<'_>>;

    #[zbus(property)]
    fn primary_connection_type(&self) -> Result<Str<'_>>;

    #[zbus(property)]
    fn wireless_enabled(&self) -> Result<bool>;
}

impl Client {
    async fn new() -> Result<Self> {
        let client_state = Mutable::new(ClientState::Unknown);
        let dbus_connection = Connection::system().await?;
        let interface_name = InterfaceName::from_static_str(DBUS_INTERFACE)?;
        let props_proxy = PropertiesProxy::builder(&dbus_connection)
            .destination(DBUS_BUS)?
            .path(DBUS_PATH)?
            .build()
            .await?;

        Ok(Self {
            client_state,
            interface_name,
            dbus_connection,
            props_proxy,
        })
    }

    async fn run(&self) -> Result<()> {
        let proxy = NetworkManagerDbusProxy::new(&self.dbus_connection).await?;

        let mut primary_connection = proxy.primary_connection().await?;
        let mut primary_connection_type = proxy.primary_connection_type().await?;
        let mut wireless_enabled = proxy.wireless_enabled().await?;

        self.client_state.set(determine_state(
            &primary_connection,
            &primary_connection_type,
            wireless_enabled,
        ));

        let mut stream = self.props_proxy.receive_properties_changed().await?;
        while let Some(change) = stream.next().await {
            let args = change.args()?;
            if args.interface_name != self.interface_name {
                continue;
            }

            let changed_props = args.changed_properties;
            let mut relevant_prop_changed = false;

            if changed_props.contains_key("PrimaryConnection") {
                primary_connection = proxy.primary_connection().await?;
                relevant_prop_changed = true;
            }
            if changed_props.contains_key("PrimaryConnectionType") {
                primary_connection_type = proxy.primary_connection_type().await?;
                relevant_prop_changed = true;
            }
            if changed_props.contains_key("WirelessEnabled") {
                wireless_enabled = proxy.wireless_enabled().await?;
                relevant_prop_changed = true;
            }

            if relevant_prop_changed {
                self.client_state.set(determine_state(
                    &primary_connection,
                    &primary_connection_type,
                    wireless_enabled,
                ));
            }
        }

        Ok(())
    }

    pub fn subscribe(&self) -> MutableSignalCloned<ClientState> {
        self.client_state.signal_cloned()
    }
}

pub async fn create_client() -> Result<Arc<Client>> {
    let client = Arc::new(Client::new().await?);
    {
        let client = client.clone();
        spawn(async move {
            if let Err(error) = client.run().await {
                error!("{}", error);
            }
        });
    }
    Ok(client)
}

fn determine_state(
    primary_connection: &str,
    primary_connection_type: &str,
    wireless_enabled: bool,
) -> ClientState {
    if primary_connection == "/" {
        if wireless_enabled {
            ClientState::WifiDisconnected
        } else {
            ClientState::Offline
        }
    } else {
        match primary_connection_type {
            "802-3-ethernet" | "adsl" | "pppoe" => ClientState::WiredConnected,
            "802-11-olpc-mesh" | "802-11-wireless" | "wifi-p2p" => ClientState::WifiConnected,
            "cdma" | "gsm" | "wimax" => ClientState::CellularConnected,
            "vpn" | "wireguard" => ClientState::VpnConnected,
            _ => ClientState::Unknown,
        }
    }
}

register_fallible_client!(Client, network_manager);
