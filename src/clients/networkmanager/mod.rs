use color_eyre::eyre::Report;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock as AsyncRwLock, broadcast};

use color_eyre::Result;
use zbus::Connection;
use zbus::zvariant::ObjectPath;

use crate::clients::networkmanager::dbus::{AccessPointDbusProxy, DbusProxy, DeviceDbusProxy};
use crate::{register_fallible_client, spawn};
use futures_lite::StreamExt;

pub use self::dbus::{DeviceState, DeviceType, DeviceWirelessDbusProxy, Ip4ConfigDbusProxy};

mod dbus;
pub mod state;

type PathMap<'l, ValueType> = HashMap<ObjectPath<'l>, ValueType>;

#[derive(Clone, Debug)]
pub enum NetworkManagerUpdate {
    /// Update all devices
    Devices(Vec<state::Device>),
    /// Update a single device.
    ///
    /// The `usize` is the index of the device in the list of devices received in the previous
    /// `Devices` update.
    Device(usize, state::Device),
}

#[derive(Debug)]
struct ClientDevice {
    state: state::Device,
    index: usize,
    state_handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug)]
struct ClientIp4Config {
    state_handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug)]
struct ClientAccessPoint {
    state_handle: tokio::task::JoinHandle<()>,
}

#[derive(Clone, Debug)]
pub struct Client {
    dbus_connection: Connection,
    root_object: &'static DbusProxy<'static>,
    tx: broadcast::Sender<NetworkManagerUpdate>,
    device_map: Arc<AsyncRwLock<HashMap<ObjectPath<'static>, ClientDevice>>>,
    ip4config_map: Arc<AsyncRwLock<PathMap<'static, ClientIp4Config>>>,
    access_point_map: Arc<AsyncRwLock<PathMap<'static, ClientAccessPoint>>>,
}
impl Client {
    async fn new() -> Result<Client> {
        let dbus_connection = Connection::system().await?;
        let root_object = {
            let root_object = DbusProxy::new(&dbus_connection).await?;
            // Workaround for the fact that zbus (unnecessarily) requires a static lifetime here
            Box::leak(Box::new(root_object))
        };

        let (tx, rx) = broadcast::channel(32);

        std::mem::forget(rx);

        Ok(Client {
            root_object,
            dbus_connection,
            tx,
            device_map: Arc::new(AsyncRwLock::new(HashMap::new())),
            ip4config_map: Arc::new(AsyncRwLock::new(HashMap::new())),
            access_point_map: Arc::new(AsyncRwLock::new(HashMap::new())),
        })
    }

    async fn fetch_devices(&self, device_paths: &[ObjectPath<'_>]) -> Result<Vec<state::Device>> {
        let mut devices = Vec::with_capacity(device_paths.len());
        for device_path in device_paths {
            let device = self.fetch_device(device_path).await?;
            devices.push(device);
        }

        devices.sort_by(|a, b| {
            fn key(d: &state::Device) -> (u32, &str) {
                (d.device_type as u32, &d.interface)
            }
            key(a).cmp(&key(b))
        });

        Ok(devices)
    }

    async fn fetch_device(&self, path: &ObjectPath<'_>) -> Result<state::Device> {
        let device_object = DeviceDbusProxy::new(&self.dbus_connection, path).await?;
        let state = device_object.state().await?;
        let device_type = device_object.device_type().await?;
        let interface = device_object.interface().await?;
        let ip4_config = if state == DeviceState::Activated {
            let ip4_config_path = device_object.ip4_config().await;
            match ip4_config_path {
                Ok(ip4_config_path) => Some(self.fetch_ip4_config(&ip4_config_path).await?),
                Err(_) => {
                    tracing::error!("Device is activated but has no IP4 config");
                    None
                }
            }
        } else {
            None
        };

        let device_type_data = self
            .fetch_device_wireless(path)
            .await
            .unwrap_or(state::DeviceTypeData::None);

        Ok(state::Device {
            path: path.to_owned(),
            interface: interface.to_string(),
            state,
            device_type,
            ip4_config,
            device_type_data,
        })
    }

    async fn fetch_ip4_config(&self, path: &ObjectPath<'_>) -> Result<state::Ip4Config> {
        let ipconfig_object = Ip4ConfigDbusProxy::new(&self.dbus_connection, path).await?;

        let address_data = ipconfig_object.address_data().await?;
        let address_data = address_data
            .iter()
            .map(|address_data| -> Result<state::AddressData> {
                let address = address_data.get("address").ok_or_else(|| {
                    Report::msg("Ip4config address data does not have field 'address'")
                })?;
                let prefix = address_data.get("prefix").ok_or_else(|| {
                    Report::msg("Ip4config address data does not have field 'prefix'")
                })?;
                let address = String::try_from(address.try_clone()?)?;
                let prefix = u32::try_from(prefix)?;
                Ok(state::AddressData { address, prefix })
            })
            .collect::<Result<Vec<state::AddressData>>>()?;

        Ok(state::Ip4Config {
            path: path.to_owned(),
            address_data,
        })
    }

    async fn fetch_device_wireless(&self, path: &ObjectPath<'_>) -> Result<state::DeviceTypeData> {
        let device_object = DeviceWirelessDbusProxy::new(&self.dbus_connection, path).await?;
        let active_access_point_path = device_object.active_access_point().await?;
        let active_access_point = if active_access_point_path.as_ref() != "/" {
            match self.fetch_access_point(&active_access_point_path).await {
                Ok(x) => Some(x),
                Err(e) => {
                    tracing::error!("failed to fetch access point: {e}");
                    None
                }
            }
        } else {
            None
        };

        Ok(state::DeviceTypeData::Wireless(state::DeviceWireless {
            active_access_point,
        }))
    }

    async fn fetch_access_point(&self, path: &ObjectPath<'_>) -> Result<state::AccessPoint> {
        let access_point_object = AccessPointDbusProxy::new(&self.dbus_connection, path).await?;
        let ssid = access_point_object.ssid().await?;
        let strength = access_point_object.strength().await?;
        Ok(state::AccessPoint {
            path: path.to_owned(),
            ssid,
            strength,
        })
    }

    async fn watch_devices_changed(&self) {
        let mut x = self.root_object.receive_devices_changed().await;
        while let Some(_change) = x.next().await {
            self.send_device_update().await;
        }
    }

    async fn send_device_update(&self) {
        let mut device_map = self.device_map.write().await;
        let mut ip4config_map = self.ip4config_map.write().await;
        let mut access_point_map = self.access_point_map.write().await;

        let device_paths = self.root_object.devices().await;
        let Ok(device_paths) = device_paths else {
            tracing::error!("Failed to get device paths");
            return;
        };
        let devices = self.fetch_devices(&device_paths).await;
        match devices {
            Ok(devices) => {
                let mut new_device_map = HashMap::new();
                let mut new_ip4config_map = HashMap::new();
                let mut new_access_point_map = HashMap::new();

                for (index, device) in devices.iter().enumerate() {
                    let path = &device.path;
                    match device_map.remove(path) {
                        Some(client_device) => {
                            let path = path.to_owned();
                            new_device_map.insert(
                                path,
                                ClientDevice {
                                    state: device.clone(),
                                    index,
                                    state_handle: client_device.state_handle,
                                },
                            );
                        }
                        None => {
                            let this = self.clone();
                            let path2 = path.to_owned();
                            let v = ClientDevice {
                                state: device.clone(),
                                index,
                                state_handle: spawn(async move {
                                    this.watch_device_change(path2).await
                                }),
                            };
                            let path = path.to_owned();
                            new_device_map.insert(path, v);
                        }
                    }

                    match device
                        .ip4_config
                        .as_ref()
                        .and_then(|ip4| ip4config_map.remove(&ip4.path))
                    {
                        Some(client_ipconfig) => {
                            if let Some(ip4config) = device.ip4_config.as_ref() {
                                new_ip4config_map
                                    .insert(ip4config.path.to_owned(), client_ipconfig);
                            }
                        }
                        None => {
                            if let Some(ip4config) = device.ip4_config.as_ref() {
                                let this = self.clone();
                                let device_path = path.to_owned();
                                let path2 = ip4config.path.to_owned();
                                let v = ClientIp4Config {
                                    state_handle: spawn(async move {
                                        this.watch_ip4config_change(device_path, path2).await
                                    }),
                                };
                                new_ip4config_map.insert(ip4config.path.to_owned(), v);
                            }
                        }
                    }

                    if let state::DeviceTypeData::Wireless(wireless) = &device.device_type_data {
                        match wireless
                            .active_access_point
                            .as_ref()
                            .and_then(|ap| access_point_map.remove(&ap.path))
                        {
                            Some(client_ap) => {
                                if let Some(ap) = wireless.active_access_point.as_ref() {
                                    new_access_point_map.insert(ap.path.to_owned(), client_ap);
                                }
                            }
                            None => {
                                if let Some(ap) = wireless.active_access_point.as_ref() {
                                    let this = self.clone();
                                    let device_path = path.to_owned();
                                    let path2 = ap.path.to_owned();
                                    let access_point_object = match AccessPointDbusProxy::new(
                                        &this.dbus_connection,
                                        &path2,
                                    )
                                    .await
                                    {
                                        Ok(proxy) => proxy,
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to create access point proxy: {e}"
                                            );
                                            continue;
                                        }
                                    };
                                    let v = ClientAccessPoint {
                                        state_handle: spawn(async move {
                                            this.watch_access_point_change(
                                                device_path,
                                                access_point_object,
                                            )
                                            .await
                                        }),
                                    };
                                    new_access_point_map.insert(ap.path.to_owned(), v);
                                }
                            }
                        }
                    }
                }

                for device in device_map.values() {
                    device.state_handle.abort();
                }

                for ipconfig in ip4config_map.values() {
                    ipconfig.state_handle.abort();
                }

                for ap in access_point_map.values() {
                    ap.state_handle.abort();
                }

                *device_map = new_device_map;
                *ip4config_map = new_ip4config_map;
                *access_point_map = new_access_point_map;

                self.tx.send(NetworkManagerUpdate::Devices(devices)).ok();
            }
            Err(e) => {
                tracing::error!("Failed to fetch devices: {e}");
            }
        }
    }

    async fn update_device(&self, device: &ObjectPath<'_>) -> Result<()> {
        let mut device_map = self.device_map.write().await;
        let entry = device_map
            .get_mut(&device.to_owned())
            .ok_or_else(|| Report::msg("Device changed but not in device map"))?;
        let device = self.fetch_device(device).await;

        match device {
            Ok(device) => {
                entry.state = device.clone();
                let index = entry.index;
                self.tx.send(NetworkManagerUpdate::Device(index, device))?;
            }
            Err(e) => {
                tracing::error!("Failed to fetch device: {e}");
            }
        }

        Ok(())
    }

    async fn watch_device_change(&self, device: ObjectPath<'_>) {
        let device = match DeviceDbusProxy::new(&self.dbus_connection, &device).await {
            Ok(device) => device,
            Err(e) => {
                tracing::error!("Failed to create device proxy: {e}");
                return;
            }
        };
        let mut state = device.receive_state_changed().await;
        let mut device_type = device.receive_device_type_changed().await;
        let mut ip4_config = device.receive_ip4_config_changed().await;

        loop {
            // Wait for any of the properties to change
            tokio::select! {
                biased;
                _ = state.next() => (),
                _ = device_type.next() => (),
                _ = ip4_config.next() => (),
            };

            let path = device.inner().path();
            match self.update_device(path).await {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Failed to update device: {e}");
                    break;
                }
            }
        }
    }

    async fn watch_ip4config_change(&self, device: ObjectPath<'_>, ip4config: ObjectPath<'_>) {
        let ip4config = match Ip4ConfigDbusProxy::new(&self.dbus_connection, &ip4config).await {
            Ok(ip4config) => ip4config,
            Err(e) => {
                tracing::error!("Failed to create ip4config proxy: {e}");
                return;
            }
        };
        let mut address_data = ip4config.receive_address_data_changed().await;
        loop {
            address_data.next().await;
            match self.update_device(&device).await {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Failed to update ip4config: {e}");
                    break;
                }
            }
        }
    }

    async fn watch_access_point_change(
        &self,
        device: ObjectPath<'_>,
        access_point: AccessPointDbusProxy<'_>,
    ) {
        let mut ssid = access_point.receive_ssid_changed().await;
        let mut strength = access_point.receive_strength_changed().await;
        loop {
            tokio::select! {
                biased;
                _ = ssid.next() => (),
                _ = strength.next() => (),
            };
            tracing::debug!("Access point changed for device {device}");
            match self.update_device(&device).await {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Failed to update access point: {e}");
                    break;
                }
            }
        }
    }

    async fn run(&self) -> Result<()> {
        let this = self.clone();
        spawn(async move { this.watch_devices_changed().await });
        Ok(())
    }

    pub async fn subscribe(self: &Arc<Self>) -> broadcast::Receiver<NetworkManagerUpdate> {
        let rx = self.tx.subscribe();
        let this = Arc::clone(self);
        spawn(async move {
            this.send_device_update().await;
        });
        rx
    }
}

pub async fn create_client() -> Result<Arc<Client>> {
    let client = Arc::new(Client::new().await?);
    {
        let client = client.clone();
        spawn(async move { client.run().await });
    }
    Ok(client)
}

register_fallible_client!(Client, network_manager);
