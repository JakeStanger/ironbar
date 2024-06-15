use std::ops::Deref;
use std::sync::{Arc, RwLock};

use color_eyre::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::error;
use zbus::blocking::Connection;
use zbus::{
    dbus_proxy,
    zvariant::{ObjectPath, Str},
};

use crate::{
    read_lock, register_fallible_client, spawn_blocking, spawn_blocking_result, write_lock,
};

// States

#[derive(Clone, Debug)]
pub struct State {
    pub wired: WiredState,
    pub wifi: WifiState,
    pub cellular: CellularState,
    pub vpn: VpnState,
}

#[derive(Clone, Debug)]
pub enum WiredState {
    Connected,
    Disconnected,
    NotPresent,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum WifiState {
    Connected(WifiConnectedState),
    Disconnected,
    Disabled,
    NotPresent,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct WifiConnectedState {
    pub ssid: String,
}

#[derive(Clone, Debug)]
pub enum CellularState {
    Connected,
    Disconnected,
    Disabled,
    NotPresent,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum VpnState {
    Connected(VpnConnectedState),
    Disconnected,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct VpnConnectedState {
    pub name: String,
}

// D-Bus interfaces

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait Dbus {
    #[dbus_proxy(property)]
    fn active_connections(&self) -> Result<Vec<ObjectPath>>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<ObjectPath>>;

    #[dbus_proxy(property)]
    fn networking_enabled(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn primary_connection(&self) -> Result<ObjectPath>;

    #[dbus_proxy(property)]
    fn primary_connection_type(&self) -> Result<Str>;

    #[dbus_proxy(property)]
    fn wireless_enabled(&self) -> Result<bool>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
trait ActiveConnectionDbus {
    #[dbus_proxy(property)]
    fn connection(&self) -> Result<ObjectPath>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<ObjectPath>>;

    #[dbus_proxy(property)]
    fn id(&self) -> Result<Str>;

    #[dbus_proxy(property)]
    fn specific_object(&self) -> Result<ObjectPath>;

    #[dbus_proxy(property)]
    fn type_(&self) -> Result<Str>;

    #[dbus_proxy(property)]
    fn uuid(&self) -> Result<Str>;
}

// Ironbar client & helpers

#[derive(Debug)]
pub struct Client(Arc<ClientInner<'static>>);

#[derive(Debug)]
struct ClientInner<'a> {
    state: Mutable<State>,

    dbus_proxy: &'a DbusProxyBlocking<'a>,

    primary_connection: RwLock<ObjectPath<'a>>,
    primary_connection_type: RwLock<Str<'a>>,
    wireless_enabled: RwLock<bool>,
}

impl Client {
    fn new() -> Result<Client> {
        let state = Mutable::new(State {
            wired: WiredState::Unknown,
            wifi: WifiState::Unknown,
            cellular: CellularState::Unknown,
            vpn: VpnState::Unknown,
        });
        let dbus_connection = Connection::system()?;
        let dbus_proxy = DbusProxyBlocking::new(&dbus_connection)?;
        let primary_connection = dbus_proxy.primary_connection()?.to_owned();
        let primary_connection_type = dbus_proxy.primary_connection_type()?.to_owned();
        let wireless_enabled = dbus_proxy.wireless_enabled()?;

        Ok(Client(Arc::new(ClientInner {
            state,
            dbus_proxy: Box::leak(Box::new(dbus_proxy)),
            primary_connection: RwLock::new(primary_connection),
            primary_connection_type: RwLock::new(primary_connection_type),
            wireless_enabled: RwLock::new(wireless_enabled),
        })))
    }

    fn run(&self) -> Result<()> {
        macro_rules! spawn_property_watcher {
            ($client:expr, $property:ident, $property_changes:ident) => {
                let client = $client.clone();
                spawn_blocking_result!({
                    while let Some(change) = client.dbus_proxy.$property_changes().next() {
                        {
                            let new_value = change.get()?;
                            let mut value_guard = write_lock!(client.$property);
                            *value_guard = new_value;
                        }
                        client.state.set(determine_state(
                            read_lock!(client.primary_connection).deref(),
                            read_lock!(client.primary_connection_type).deref(),
                            *read_lock!(client.wireless_enabled),
                        ));
                    }
                    Ok(())
                });
            };
        }

        // Initial state
        self.0.state.set(determine_state(
            &read_lock!(self.0.primary_connection),
            &read_lock!(self.0.primary_connection_type),
            *read_lock!(self.0.wireless_enabled),
        ));

        spawn_property_watcher!(
            self.0,
            primary_connection,
            receive_primary_connection_changed
        );
        spawn_property_watcher!(
            self.0,
            primary_connection_type,
            receive_primary_connection_type_changed
        );
        spawn_property_watcher!(self.0, wireless_enabled, receive_wireless_enabled_changed);

        Ok(())
    }

    pub fn subscribe(&self) -> MutableSignalCloned<State> {
        self.0.state.signal_cloned()
    }
}

pub fn create_client() -> Result<Arc<Client>> {
    let client = Arc::new(Client::new()?);
    {
        let client = client.clone();
        spawn_blocking_result!({
            client.run()?;
            Ok(())
        });
    }
    Ok(client)
}

fn determine_state(
    primary_connection: &ObjectPath,
    primary_connection_type: &Str,
    wireless_enabled: bool,
) -> State {
    if primary_connection == "/" {
        if wireless_enabled {
            State {
                wired: WiredState::Unknown,
                wifi: WifiState::Disconnected,
                cellular: CellularState::Unknown,
                vpn: VpnState::Unknown,
            }
        } else {
            State {
                wired: WiredState::Unknown,
                wifi: WifiState::Disabled,
                cellular: CellularState::Unknown,
                vpn: VpnState::Unknown,
            }
        }
    } else {
        match primary_connection_type.as_str() {
            "802-3-ethernet" | "adsl" | "pppoe" => State {
                wired: WiredState::Connected,
                wifi: WifiState::Unknown,
                cellular: CellularState::Unknown,
                vpn: VpnState::Unknown,
            },
            "802-11-olpc-mesh" | "802-11-wireless" | "wifi-p2p" => State {
                wired: WiredState::Unknown,
                wifi: WifiState::Connected(WifiConnectedState {
                    ssid: String::new(),
                }),
                cellular: CellularState::Unknown,
                vpn: VpnState::Unknown,
            },
            "cdma" | "gsm" | "wimax" => State {
                wired: WiredState::Unknown,
                wifi: WifiState::Unknown,
                cellular: CellularState::Connected,
                vpn: VpnState::Unknown,
            },
            "vpn" | "wireguard" => State {
                wired: WiredState::Unknown,
                wifi: WifiState::Unknown,
                cellular: CellularState::Unknown,
                vpn: VpnState::Connected(VpnConnectedState {
                    name: String::new(),
                }),
            },
            _ => State {
                wired: WiredState::Unknown,
                wifi: WifiState::Unknown,
                cellular: CellularState::Unknown,
                vpn: VpnState::Unknown,
            },
        }
    }
}

// fn instantiate_active_connections<'a>(
//     dbus_connection: &Connection,
//     active_connection_paths: Vec<ObjectPath>,
// ) -> Result<Vec<ActiveConnectionDbusProxyBlocking<'a>>> {
//     let mut active_connections = Vec::new();
//     for active_connection_path in active_connection_paths {
//         let active_connection_proxy = ActiveConnectionDbusProxyBlocking::builder(dbus_connection)
//             .path(active_connection_path)?
//             .build()?;
//         active_connections.push(active_connection_proxy);
//     }
//     Ok(active_connections)
// }

register_fallible_client!(Client, networkmanager);
