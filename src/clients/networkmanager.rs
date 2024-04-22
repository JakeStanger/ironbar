use std::sync::Arc;

use color_eyre::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::error;
use zbus::{
    blocking::{fdo::PropertiesProxy, Connection},
    names::InterfaceName,
    zvariant::{Error as ZVariantError, ObjectPath, Str},
    Error as ZBusError,
};

use crate::{register_fallible_client, spawn_blocking};

static DBUS_BUS: &str = "org.freedesktop.NetworkManager";
static DBUS_PATH: &str = "/org/freedesktop/NetworkManager";
static DBUS_INTERFACE: &str = "org.freedesktop.NetworkManager";

#[derive(Debug)]
pub struct Client {
    client_state: Mutable<ClientState>,
    interface_name: InterfaceName<'static>,
    props_proxy: PropertiesProxy<'static>,
}

#[derive(Clone, Debug)]
pub enum ClientState {
    Unknown,
    WiredConnected,
    WifiConnected,
    CellularConnected,
    VpnConnected,
    WifiDisconnected,
    Offline,
}

impl Client {
    fn new() -> Result<Self> {
        let client_state = Mutable::new(ClientState::Unknown);
        let dbus_connection = Connection::system()?;
        let props_proxy = PropertiesProxy::builder(&dbus_connection)
            .destination(DBUS_BUS)?
            .path(DBUS_PATH)?
            .build()?;
        let interface_name = InterfaceName::from_static_str(DBUS_INTERFACE)?;

        Ok(Self {
            client_state,
            interface_name,
            props_proxy,
        })
    }

    fn run(&self) -> Result<()> {
        let props = self.props_proxy.get_all(self.interface_name.clone())?;
        let mut primary_connection = props["PrimaryConnection"]
            .downcast_ref::<ObjectPath>()
            .ok_or(ZBusError::Variant(ZVariantError::IncorrectType))?
            .to_string();
        let mut primary_connection_type = props["PrimaryConnectionType"]
            .downcast_ref::<Str>()
            .ok_or(ZBusError::Variant(ZVariantError::IncorrectType))?
            .to_string();
        let mut wireless_enabled = *props["WirelessEnabled"]
            .downcast_ref::<bool>()
            .ok_or(ZBusError::Variant(ZVariantError::IncorrectType))?;
        self.client_state.set(determine_state(
            &primary_connection,
            &primary_connection_type,
            wireless_enabled,
        ));

        let changed_props_stream = self.props_proxy.receive_properties_changed()?;
        for signal in changed_props_stream {
            let args = signal.args()?;
            if args.interface_name != self.interface_name {
                continue;
            }
            let changed_props = args.changed_properties;
            if let Some(new_primary_connection) = changed_props.get("PrimaryConnection") {
                let new_primary_connection = new_primary_connection
                    .downcast_ref::<ObjectPath>()
                    .ok_or(ZBusError::Variant(ZVariantError::IncorrectType))?;
                primary_connection = new_primary_connection.to_string();
            }
            if let Some(new_primary_connection_type) = changed_props.get("PrimaryConnectionType") {
                let new_primary_connection_type = new_primary_connection_type
                    .downcast_ref::<Str>()
                    .ok_or(ZBusError::Variant(ZVariantError::IncorrectType))?;
                primary_connection_type = new_primary_connection_type.to_string();
            }
            if let Some(new_wireless_enabled) = changed_props.get("WirelessEnabled") {
                let new_wireless_enabled = new_wireless_enabled
                    .downcast_ref::<bool>()
                    .ok_or(ZBusError::Variant(ZVariantError::IncorrectType))?;
                wireless_enabled = *new_wireless_enabled;
            }
            self.client_state.set(determine_state(
                &primary_connection,
                &primary_connection_type,
                wireless_enabled,
            ));
        }

        Ok(())
    }

    pub fn subscribe(&self) -> MutableSignalCloned<ClientState> {
        self.client_state.signal_cloned()
    }
}

pub fn create_client() -> Result<Arc<Client>> {
    let client = Arc::new(Client::new()?);
    {
        let client = client.clone();
        spawn_blocking(move || {
            if let Err(error) = client.run() {
                error!("{}", error)
            };
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
            "802-11-olpc-mesh" => ClientState::WifiConnected,
            "802-11-wireless" => ClientState::WifiConnected,
            "802-3-ethernet" => ClientState::WiredConnected,
            "adsl" => ClientState::WiredConnected,
            "cdma" => ClientState::CellularConnected,
            "gsm" => ClientState::CellularConnected,
            "pppoe" => ClientState::WiredConnected,
            "vpn" => ClientState::VpnConnected,
            "wifi-p2p" => ClientState::WifiConnected,
            "wimax" => ClientState::CellularConnected,
            "wireguard" => ClientState::VpnConnected,
            _ => ClientState::Unknown,
        }
    }
}

register_fallible_client!(Client, networkmanager);
