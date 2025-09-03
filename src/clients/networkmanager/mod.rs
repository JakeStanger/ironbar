use color_eyre::Result;
use color_eyre::eyre::Ok;
use futures_lite::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;
use tracing::debug;
use zbus::Connection;
use zbus::zvariant::ObjectPath;

use crate::clients::ClientResult;
use crate::clients::networkmanager::dbus::{DbusProxy, DeviceDbusProxy};
use crate::clients::networkmanager::event::{ClientToModuleEvent, ModuleToClientEvent};
use crate::{register_fallible_client, spawn};

pub mod dbus;
pub mod event;

#[derive(Debug)]
pub struct Client {
    inner: &'static ClientInner,
}

impl Client {
    fn new() -> Client {
        let inner = Box::leak(Box::new(ClientInner::new()));
        Client { inner }
    }

    fn run(&self) -> Result<()> {
        self.inner.run()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ClientToModuleEvent> {
        self.inner.subscribe()
    }

    pub fn get_sender(&self) -> broadcast::Sender<ModuleToClientEvent> {
        self.inner.get_sender()
    }
}

#[derive(Debug)]
struct ClientInner {
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
    sender: broadcast::Sender<ModuleToClientEvent>,
    devices: RwLock<HashSet<ObjectPath<'static>>>,
    watchers: RwLock<HashMap<ObjectPath<'static>, Device>>,
    // TODO: Maybe find some way to late-init a dbus connection here
    //     so we can just clone it when we need it instead of awaiting it every time
}

#[derive(Debug)]
struct Device {
    state_watcher: JoinHandle<Result<()>>,
}

impl ClientInner {
    fn new() -> ClientInner {
        let (controller_sender, _) = broadcast::channel(64);
        let (sender, _) = broadcast::channel(8);
        let devices = RwLock::new(HashSet::new());
        let watchers = RwLock::new(HashMap::new());
        ClientInner {
            controller_sender,
            sender,
            devices,
            watchers,
        }
    }

    fn run(&'static self) -> Result<()> {
        debug!("Client running");

        spawn(self.watch_devices_list());

        let receiver = self.sender.subscribe();
        spawn(self.handle_received_events(receiver));

        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ClientToModuleEvent> {
        self.controller_sender.subscribe()
    }

    fn get_sender(&self) -> broadcast::Sender<ModuleToClientEvent> {
        self.sender.clone()
    }

    async fn watch_devices_list(&'static self) -> Result<()> {
        debug!("D-Bus devices list watcher starting");

        let dbus_connection = Connection::system().await?;
        let root = DbusProxy::new(&dbus_connection).await?;

        let mut devices_changes = root.receive_all_devices_changed().await;
        while let Some(devices_change) = devices_changes.next().await {
            // The new list of devices from dbus, not to be confused with the added devices below
            let new_devices = devices_change
                .get()
                .await?
                .iter()
                .map(ObjectPath::to_owned)
                .collect::<HashSet<_>>();

            // TODO: Use `self.watchers` instead of `self.devices`, which requires creating all property watchers straightaway

            // Atomic read-then-write of `devices`
            let mut devices_locked = self.devices.write().await;
            let devices_snapshot = devices_locked.clone();
            (*devices_locked).clone_from(&new_devices);
            drop(devices_locked);

            let added_devices = new_devices.difference(&devices_snapshot);
            for added_device in added_devices {
                spawn(self.watch_device(added_device.to_owned()));
            }

            // TODO: Inform module of removed devices
            let removed_devices = devices_snapshot.difference(&new_devices);
            for removed_device in removed_devices {
                let mut watchers = self.watchers.write().await;
                let device = watchers.get(removed_device).unwrap();
                device.state_watcher.abort();
                watchers.remove(removed_device);

                debug!("D-bus device state watcher for {} stopped", removed_device);
            }
        }

        Ok(())
    }

    async fn handle_received_events(
        &'static self,
        mut receiver: broadcast::Receiver<ModuleToClientEvent>,
    ) -> Result<()> {
        let dbus_connection = Connection::system().await?;

        while let Result::Ok(event) = receiver.recv().await {
            match event {
                ModuleToClientEvent::NewController => {
                    debug!("Client received NewController event");

                    // We create a local clone here to avoid holding the lock for too long
                    let devices_snapshot = self.devices.read().await.clone();

                    for device_path in devices_snapshot {
                        let device = DeviceDbusProxy::new(&dbus_connection, device_path).await?;

                        let interface = device.interface().await?.to_string();
                        let r#type = device.device_type().await?;
                        let new_state = device.state().await?;
                        self.controller_sender
                            .send(ClientToModuleEvent::DeviceChanged {
                                interface,
                                r#type,
                                new_state,
                            })?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn watch_device(&'static self, path: ObjectPath<'static>) -> Result<()> {
        debug_assert!(!self.watchers.read().await.contains_key(&path));

        let state_watcher = spawn(self.watch_device_state(path.clone()));
        self.watchers
            .write()
            .await
            .insert(path, Device { state_watcher });

        Ok(())
    }

    async fn watch_device_state(&'static self, path: ObjectPath<'_>) -> Result<()> {
        let dbus_connection = Connection::system().await?;
        let device = DeviceDbusProxy::new(&dbus_connection, path.clone()).await?;

        debug!("D-Bus device state watcher for {} starting", path);

        let interface = device.interface().await?;
        let r#type = device.device_type().await?;

        // Send an event communicating the initial state
        let new_state = device.state().await?;
        self.controller_sender
            .send(ClientToModuleEvent::DeviceChanged {
                interface: interface.to_string(),
                r#type: r#type.clone(),
                new_state,
            })?;

        let mut state_changes = device.receive_state_changed().await;
        while let Some(state_change) = state_changes.next().await {
            let new_state = state_change.get().await?;
            self.controller_sender
                .send(ClientToModuleEvent::DeviceChanged {
                    interface: interface.to_string(),
                    r#type: r#type.clone(),
                    new_state,
                })?;
        }

        debug!("D-Bus device state watcher for {} ended", path);

        Ok(())
    }
}

pub fn create_client() -> ClientResult<Client> {
    let client = Arc::new(Client::new());
    client.run()?;
    Ok(client)
}

register_fallible_client!(Client, network_manager);
