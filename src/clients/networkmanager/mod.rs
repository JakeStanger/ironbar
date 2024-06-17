mod dbus;
pub mod state;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use color_eyre::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::error;
use zbus::blocking::Connection;
use zbus::zvariant::ObjectPath;

use crate::clients::networkmanager::dbus::{
    ActiveConnectionDbusProxyBlocking, DbusProxyBlocking, DeviceDbusProxyBlocking,
};
use crate::clients::networkmanager::state::{
    determine_cellular_state, determine_vpn_state, determine_wifi_state, determine_wired_state,
    CellularState, State, VpnState, WifiState, WiredState,
};
use crate::{
    read_lock, register_fallible_client, spawn_blocking, spawn_blocking_result, write_lock,
};

type PathMap<'a, T> = HashMap<ObjectPath<'a>, T>;

#[derive(Debug)]
pub struct Client(Arc<ClientInner<'static>>);

#[derive(Debug)]
struct ClientInner<'a> {
    state: Mutable<State>,
    root_object: &'a DbusProxyBlocking<'a>,
    active_connections: RwLock<PathMap<'a, ActiveConnectionDbusProxyBlocking<'a>>>,
    devices: RwLock<PathMap<'a, DeviceDbusProxyBlocking<'a>>>,
    dbus_connection: Connection,
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

        Ok(Client(Arc::new(ClientInner {
            state,
            root_object: Box::leak(Box::new(dbus_proxy)), // TODO: Check if boxing is still necessary
            active_connections: RwLock::new(HashMap::new()),
            devices: RwLock::new(HashMap::new()),
            dbus_connection,
        })))
    }

    fn run(&self) -> Result<()> {
        macro_rules! spawn_property_watcher {
            ($client:expr, $property_changes:ident) => {
                let client = $client.clone();
                spawn_blocking_result!({
                    while let Some(_) = client.root_object.$property_changes().next() {
                        client.state.set(State {
                            wired: determine_wired_state(&read_lock!(client.devices))?,
                            wifi: determine_wifi_state(&read_lock!(client.devices))?,
                            cellular: determine_cellular_state(&read_lock!(client.devices))?,
                            vpn: determine_vpn_state(&read_lock!(client.active_connections))?,
                        });
                    }
                    Ok(())
                });
            };
        }

        macro_rules! spawn_list_watcher {
            () => {};
        }

        // Initial state
        self.0.state.set(State {
            wired: determine_wired_state(&read_lock!(self.0.devices))?,
            wifi: determine_wifi_state(&read_lock!(self.0.devices))?,
            cellular: determine_cellular_state(&read_lock!(self.0.devices))?,
            vpn: determine_vpn_state(&read_lock!(self.0.active_connections))?,
        });

        // Active connections paths list watcher
        {
            let client = self.0.clone();
            spawn_blocking_result!({
                let mut changes = client.root_object.receive_active_connections_changed();
                while let Some(_) = changes.next() {
                    let mut new_pathmap = HashMap::new();
                    {
                        let new_paths = client.root_object.active_connections()?;
                        let active_connections = read_lock!(client.active_connections);
                        for new_path in new_paths {
                            if active_connections.contains_key(&new_path) {
                                let proxy = active_connections
                                    .get(&new_path)
                                    .expect("Should contain the key, see check above");
                                new_pathmap.insert(new_path, proxy.clone());
                            } else {
                                let new_proxy = ActiveConnectionDbusProxyBlocking::builder(
                                    &client.dbus_connection,
                                )
                                .path(new_path.clone())?
                                .build()?;
                                new_pathmap.insert(new_path, new_proxy);

                                // Active connection type is assumed to never change
                            }
                        }
                    }
                    *write_lock!(client.active_connections) = new_pathmap;
                    client.state.set(State {
                        wired: determine_wired_state(&read_lock!(client.devices))?,
                        wifi: determine_wifi_state(&read_lock!(client.devices))?,
                        cellular: determine_cellular_state(&read_lock!(client.devices))?,
                        vpn: determine_vpn_state(&read_lock!(client.active_connections))?,
                    });
                }
                Ok(())
            });
        }

        // Devices paths list watcher
        {
            let client = self.0.clone();
            spawn_blocking_result!({
                let mut changes = client.root_object.receive_devices_changed();
                while let Some(_) = changes.next() {
                    let mut new_pathmap = HashMap::new();
                    {
                        let new_paths = client.root_object.devices()?;
                        let devices = read_lock!(client.devices);
                        for new_path in new_paths {
                            if devices.contains_key(&new_path) {
                                let proxy = devices
                                    .get(&new_path)
                                    .expect("Should contain the key, see check above");
                                new_pathmap.insert(new_path, proxy.clone());
                            } else {
                                let new_proxy =
                                    DeviceDbusProxyBlocking::builder(&client.dbus_connection)
                                        .path(new_path.clone())?
                                        .build()?;

                                // Specific device state watcher
                                {
                                    let client = client.clone();
                                    let new_path = new_path.clone();
                                    spawn_blocking_result!({
                                        let mut changes = read_lock!(client.devices)
                                            .get(&new_path)
                                            .unwrap()
                                            .receive_state_changed();
                                        while let Some(_) = changes.next() {
                                            client.state.set(State {
                                                wired: determine_wired_state(&read_lock!(
                                                    client.devices
                                                ))?,
                                                wifi: determine_wifi_state(&read_lock!(
                                                    client.devices
                                                ))?,
                                                cellular: determine_cellular_state(&read_lock!(
                                                    client.devices
                                                ))?,
                                                vpn: determine_vpn_state(&read_lock!(
                                                    client.active_connections
                                                ))?,
                                            });
                                        }
                                        Ok(())
                                    });
                                }

                                // Device type is assumed to never change

                                new_pathmap.insert(new_path, new_proxy);
                            }
                        }
                    }
                    *write_lock!(client.devices) = new_pathmap;
                    client.state.set(State {
                        wired: determine_wired_state(&read_lock!(client.devices))?,
                        wifi: determine_wifi_state(&read_lock!(client.devices))?,
                        cellular: determine_cellular_state(&read_lock!(client.devices))?,
                        vpn: determine_vpn_state(&read_lock!(client.active_connections))?,
                    });
                }
                Ok(())
            });
        }

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

register_fallible_client!(Client, networkmanager);
