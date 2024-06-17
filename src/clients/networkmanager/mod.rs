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

mod dbus;
pub mod state;

type PathMap<'l, ValueType> = HashMap<ObjectPath<'l>, ValueType>;

#[derive(Debug)]
pub struct Client(Arc<ClientInner<'static>>);

#[derive(Debug)]
struct ClientInner<'l> {
    state: Mutable<State>,
    root_object: &'l DbusProxyBlocking<'l>,
    active_connections: RwLock<PathMap<'l, ActiveConnectionDbusProxyBlocking<'l>>>,
    devices: RwLock<PathMap<'l, DeviceDbusProxyBlocking<'l>>>,
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
        let root_object = {
            let root_object = DbusProxyBlocking::new(&dbus_connection)?;
            // Workaround for the fact that zbus (unnecessarily) requires a static lifetime here
            Box::leak(Box::new(root_object))
        };

        Ok(Client(Arc::new(ClientInner {
            state,
            root_object,
            active_connections: RwLock::new(HashMap::new()),
            devices: RwLock::new(HashMap::new()),
            dbus_connection,
        })))
    }

    fn run(&self) -> Result<()> {
        // Initialisation
        {
            let client = &self.0;

            // Initial active connections path list
            {
                let new_paths = client.root_object.active_connections()?;
                let mut pathmap = write_lock!(client.active_connections);
                for new_path in new_paths {
                    let new_proxy =
                        ActiveConnectionDbusProxyBlocking::builder(&client.dbus_connection)
                            .path(new_path.clone())?
                            .build()?;
                    pathmap.insert(new_path, new_proxy);
                }
            }

            // Initial devices path list
            {
                let new_paths = client.root_object.devices()?;
                let mut pathmap = write_lock!(client.devices);
                for new_path in new_paths {
                    let new_proxy = DeviceDbusProxyBlocking::builder(&client.dbus_connection)
                        .path(new_path.clone())?
                        .build()?;

                    // Specific device state watcher
                    {
                        let client = client.clone();
                        let new_path = new_path.clone();
                        spawn_blocking_result!({
                            let changes = read_lock!(client.devices)
                                .get(&new_path)
                                .unwrap()
                                .receive_state_changed();
                            for _ in changes {
                                // TODO: Check if our device still exists in client.devices
                                client.state.set(State {
                                    wired: determine_wired_state(&read_lock!(client.devices))?,
                                    wifi: determine_wifi_state(&read_lock!(client.devices))?,
                                    cellular: determine_cellular_state(&read_lock!(
                                        client.devices
                                    ))?,
                                    vpn: client.state.get_cloned().vpn,
                                });
                            }
                            Ok(())
                        });
                    }

                    pathmap.insert(new_path, new_proxy);
                }
            }

            client.state.set(State {
                wired: determine_wired_state(&read_lock!(client.devices))?,
                wifi: determine_wifi_state(&read_lock!(client.devices))?,
                cellular: determine_cellular_state(&read_lock!(client.devices))?,
                vpn: determine_vpn_state(&read_lock!(client.active_connections))?,
            });
        }

        // Watcher for active connections path list
        {
            let client = self.0.clone();
            spawn_blocking_result!({
                let changes = client.root_object.receive_active_connections_changed();
                for _ in changes {
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
                        wired: client.state.get_cloned().wired,
                        wifi: client.state.get_cloned().wifi,
                        cellular: client.state.get_cloned().cellular,
                        vpn: determine_vpn_state(&read_lock!(client.active_connections))?,
                    });
                }
                Ok(())
            });
        }

        // Watcher for devices path list
        {
            let client = self.0.clone();
            spawn_blocking_result!({
                let changes = client.root_object.receive_devices_changed();
                for _ in changes {
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
                                        let changes = read_lock!(client.devices)
                                            .get(&new_path)
                                            .unwrap()
                                            .receive_state_changed();
                                        for _ in changes {
                                            // TODO: Check if our device still exists in client.devices
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
                                                vpn: client.state.get_cloned().vpn,
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
                        vpn: client.state.get_cloned().vpn,
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
