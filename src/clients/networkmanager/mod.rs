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
        macro_rules! update_state_for_device_change {
            ($client:ident) => {
                $client.state.set(State {
                    wired: determine_wired_state(&read_lock!($client.devices))?,
                    wifi: determine_wifi_state(&read_lock!($client.devices))?,
                    cellular: determine_cellular_state(&read_lock!($client.devices))?,
                    vpn: $client.state.get_cloned().vpn,
                });
            };
        }

        macro_rules! initialise_path_map {
            (
                $client:expr,
                $path_map:ident,
                $proxy_type:ident
                $(, |$new_path:ident| $property_watcher:expr)*
            ) => {
                let new_paths = $client.root_object.$path_map()?;
                let mut path_map = HashMap::new();
                for new_path in new_paths {
                    let new_proxy = $proxy_type::builder(&$client.dbus_connection)
                        .path(new_path.clone())?
                        .build()?;
                    path_map.insert(new_path.clone(), new_proxy);
                    $({
                        let $new_path = &new_path;
                        $property_watcher;
                    })*
                }
                *write_lock!($client.$path_map) = path_map;
            };
        }

        macro_rules! spawn_path_list_watcher {
            (
                $client:expr,
                $property:ident,
                $property_changes:ident,
                $proxy_type:ident,
                |$state_client:ident| $state_update:expr
                $(, |$property_client:ident, $new_path:ident| $property_watcher:expr)*
            ) => {
                let client = $client.clone();
                spawn_blocking_result!({
                    let changes = client.root_object.$property_changes();
                    for _ in changes {
                        let mut new_path_map = HashMap::new();
                        {
                            let new_paths = client.root_object.$property()?;
                            let path_map = read_lock!(client.$property);
                            for new_path in new_paths {
                                if path_map.contains_key(&new_path) {
                                    let proxy = path_map
                                        .get(&new_path)
                                        .expect("Should contain the key, guarded by runtime check");
                                    new_path_map.insert(new_path, proxy.to_owned());
                                } else {
                                    let new_proxy = $proxy_type::builder(&client.dbus_connection)
                                        .path(new_path.clone())?
                                        .build()?;
                                    new_path_map.insert(new_path.clone(), new_proxy);
                                    $({
                                        let $property_client = &client;
                                        let $new_path = &new_path;
                                        $property_watcher;
                                    })*
                                }
                            }
                        }
                        *write_lock!(client.$property) = new_path_map;
                        {
                            let $state_client = &client;
                            $state_update;
                        }
                    }
                    Ok(())
                });
            }
        }

        macro_rules! spawn_property_watcher {
            (
                $client:expr,
                $path:expr,
                $property_changes:ident,
                $containing_list:ident,
                |$inner_client:ident| $state_update:expr
            ) => {
                let client = $client.clone();
                let path = $path.clone();
                spawn_blocking_result!({
                    let changes = read_lock!(client.$containing_list)
                        .get(&path)
                        .expect("Should contain the key upon watcher start")
                        .$property_changes();
                    for _ in changes {
                        if !read_lock!(client.$containing_list).contains_key(&path) {
                            break;
                        }
                        {
                            let $inner_client = &client;
                            $state_update;
                        }
                    }
                    Ok(())
                });
            };
        }

        initialise_path_map!(
            self.0,
            active_connections,
            ActiveConnectionDbusProxyBlocking
        );
        initialise_path_map!(self.0, devices, DeviceDbusProxyBlocking, |path| {
            spawn_property_watcher!(self.0, path, receive_state_changed, devices, |client| {
                update_state_for_device_change!(client);
            });
        });
        self.0.state.set(State {
            wired: determine_wired_state(&read_lock!(self.0.devices))?,
            wifi: determine_wifi_state(&read_lock!(self.0.devices))?,
            cellular: determine_cellular_state(&read_lock!(self.0.devices))?,
            vpn: determine_vpn_state(&read_lock!(self.0.active_connections))?,
        });

        spawn_path_list_watcher!(
            self.0,
            active_connections,
            receive_active_connections_changed,
            ActiveConnectionDbusProxyBlocking,
            |client| {
                client.state.set(State {
                    wired: client.state.get_cloned().wired,
                    wifi: client.state.get_cloned().wifi,
                    cellular: client.state.get_cloned().cellular,
                    vpn: determine_vpn_state(&read_lock!(client.active_connections))?,
                });
            }
        );
        spawn_path_list_watcher!(
            self.0,
            devices,
            receive_devices_changed,
            DeviceDbusProxyBlocking,
            |client| {
                update_state_for_device_change!(client);
            },
            |client, path| {
                spawn_property_watcher!(client, path, receive_state_changed, devices, |client| {
                    update_state_for_device_change!(client);
                });
            }
        );

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
