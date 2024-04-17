use std::sync::Arc;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::error;
use zbus::{
    blocking::{fdo::PropertiesProxy, Connection},
    names::InterfaceName,
    zvariant::{ObjectPath, Str},
};

use crate::{register_client, spawn_blocking};

static DBUS_BUS: &str = "org.freedesktop.NetworkManager";
static DBUS_PATH: &str = "/org/freedesktop/NetworkManager";
static DBUS_INTERFACE: &str = "org.freedesktop.NetworkManager";

#[derive(Debug)]
pub struct Client {
    client_state: Mutable<ClientState>,
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
    fn new() -> Self {
        let client_state = Mutable::new(ClientState::Unknown);
        Self { client_state }
    }

    fn run(&self) {
        let Ok(dbus_connection) = Connection::system() else {
            error!("Failed to create D-Bus system connection");
            return;
        };
        let builder = PropertiesProxy::builder(&dbus_connection);
        let Ok(builder) = builder.destination(DBUS_BUS) else {
            error!("Failed to connect to NetworkManager D-Bus bus");
            return;
        };
        let Ok(builder) = builder.path(DBUS_PATH) else {
            error!("Failed to set to NetworkManager D-Bus path");
            return;
        };
        let Ok(props_proxy) = builder.build() else {
            error!("Failed to create NetworkManager D-Bus properties proxy");
            return;
        };
        let Ok(interface_name) = InterfaceName::from_static_str(DBUS_INTERFACE) else {
            error!("Failed to create NetworkManager D-Bus interface name");
            return;
        };
        let Ok(changed_props_stream) = props_proxy.receive_properties_changed() else {
            error!("Failed to create NetworkManager D-Bus changed properties stream");
            return;
        };
        let Ok(props) = props_proxy.get_all(interface_name.clone()) else {
            error!("Failed to get NetworkManager D-Bus properties");
            return;
        };

        let mut primary_connection = {
            let Some(primary_connection) = props["PrimaryConnection"].downcast_ref::<ObjectPath>()
            else {
                error!("PrimaryConnection D-Bus property is not a path");
                return;
            };
            primary_connection.to_string()
        };
        let mut primary_connection_type = {
            let Some(primary_connection_type) =
                props["PrimaryConnectionType"].downcast_ref::<Str>()
            else {
                error!("PrimaryConnectionType D-Bus property is not a string");
                return;
            };
            primary_connection_type.to_string()
        };
        let mut wireless_enabled = {
            let Some(wireless_enabled) = props["WirelessEnabled"].downcast_ref::<bool>() else {
                error!("WirelessEnabled D-Bus property is not a boolean");
                return;
            };
            *wireless_enabled
        };
        self.client_state.set(determine_state(
            &primary_connection,
            &primary_connection_type,
            wireless_enabled,
        ));

        for signal in changed_props_stream {
            let Ok(args) = signal.args() else {
                error!("Failed to obtain NetworkManager D-Bus changed properties signal arguments");
                return;
            };
            if args.interface_name != interface_name {
                continue;
            }
            let changed_props = args.changed_properties;
            if let Some(new_primary_connection) = changed_props.get("PrimaryConnection") {
                let Some(new_primary_connection) =
                    new_primary_connection.downcast_ref::<ObjectPath>()
                else {
                    error!("PrimaryConnection D-Bus property is not a path");
                    return;
                };
                primary_connection = new_primary_connection.to_string();
            }
            if let Some(new_primary_connection_type) = changed_props.get("PrimaryConnectionType") {
                let Some(new_primary_connection_type) =
                    new_primary_connection_type.downcast_ref::<Str>()
                else {
                    error!("PrimaryConnectionType D-Bus property is not a string");
                    return;
                };
                primary_connection_type = new_primary_connection_type.to_string();
            }
            if let Some(new_wireless_enabled) = changed_props.get("WirelessEnabled") {
                let Some(new_wireless_enabled) = new_wireless_enabled.downcast_ref::<bool>() else {
                    error!("WirelessEnabled D-Bus property is not a string");
                    return;
                };
                wireless_enabled = *new_wireless_enabled;
            }
            self.client_state.set(determine_state(
                &primary_connection,
                &primary_connection_type,
                wireless_enabled,
            ));
        }
    }

    pub fn subscribe(&self) -> MutableSignalCloned<ClientState> {
        self.client_state.signal_cloned()
    }
}

pub fn create_client() -> Arc<Client> {
    let client = Arc::new(Client::new());
    {
        let client = client.clone();
        spawn_blocking(move || {
            client.run();
        });
    }
    client
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

register_client!(Client, networkmanager);
