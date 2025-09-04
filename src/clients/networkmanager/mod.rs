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
    device_watchers: RwLock<HashMap<ObjectPath<'static>, DeviceWatcher>>,
    dbus_connection: RwLock<Option<Connection>>,
}

#[derive(Clone, Debug)]
struct DeviceWatcher {
    state_watcher: Arc<JoinHandle<Result<()>>>,
}

impl ClientInner {
    fn new() -> ClientInner {
        let (controller_sender, _) = broadcast::channel(64);
        let (sender, _) = broadcast::channel(8);
        let device_watchers = RwLock::new(HashMap::new());
        let dbus_connection = RwLock::new(None);
        ClientInner {
            controller_sender,
            sender,
            device_watchers,
            dbus_connection,
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

        let root = DbusProxy::new(&self.dbus_connection().await?).await?;

        let mut devices_changes = root.receive_all_devices_changed().await;
        while let Some(devices_change) = devices_changes.next().await {
            // The new list of devices from dbus, not to be confused with the added devices below
            let new_device_paths = devices_change
                .get()
                .await?
                .iter()
                .map(ObjectPath::to_owned)
                .collect::<HashSet<_>>();

            let mut watchers = self.device_watchers.write().await;
            let device_paths = watchers.keys().cloned().collect::<HashSet<_>>();

            let added_device_paths = new_device_paths.difference(&device_paths);
            for added_device_path in added_device_paths {
                debug_assert!(!watchers.contains_key(added_device_path));

                let watcher = self.watch_device(added_device_path.clone());
                watchers.insert(added_device_path.clone(), watcher);
            }

            let removed_device_paths = device_paths.difference(&new_device_paths);
            for removed_device_path in removed_device_paths {
                let watcher = watchers
                    .get(removed_device_path)
                    .expect("Device to be removed should be present in watchers");
                watcher.state_watcher.abort();
                watchers.remove(removed_device_path);

                let number = get_number_from_dbus_path(removed_device_path);
                self.controller_sender
                    .send(ClientToModuleEvent::DeviceRemoved { number })?;

                debug!("D-bus device watchers for {} stopped", removed_device_path);
            }
        }

        Ok(())
    }

    async fn handle_received_events(
        &'static self,
        mut receiver: broadcast::Receiver<ModuleToClientEvent>,
    ) -> Result<()> {
        while let Result::Ok(event) = receiver.recv().await {
            match event {
                ModuleToClientEvent::NewController => {
                    debug!("Client received NewController event");

                    for device_path in self.device_watchers.read().await.keys() {
                        let dbus_connection = &self.dbus_connection().await?;
                        let device = DeviceDbusProxy::new(dbus_connection, device_path).await?;

                        let number = get_number_from_dbus_path(device_path);
                        let r#type = device.device_type().await?;
                        let new_state = device.state().await?;
                        self.controller_sender
                            .send(ClientToModuleEvent::DeviceChanged {
                                number,
                                r#type,
                                new_state,
                            })?;
                    }
                }
            }
        }

        Ok(())
    }

    fn watch_device(&'static self, path: ObjectPath<'static>) -> DeviceWatcher {
        let state_watcher = Arc::new(spawn(self.watch_device_state(path)));

        DeviceWatcher { state_watcher }
    }

    async fn watch_device_state(&'static self, path: ObjectPath<'_>) -> Result<()> {
        debug!("D-Bus device state watcher for {} starting", path);

        let dbus_connection = Connection::system().await?;
        let device = DeviceDbusProxy::new(&dbus_connection, path.clone()).await?;

        let number = get_number_from_dbus_path(&path);
        let r#type = device.device_type().await?;

        // Send an event communicating the initial state
        let new_state = device.state().await?;
        self.controller_sender
            .send(ClientToModuleEvent::DeviceChanged {
                number,
                r#type: r#type.clone(),
                new_state,
            })?;

        let mut state_changes = device.receive_state_changed().await;
        while let Some(state_change) = state_changes.next().await {
            let new_state = state_change.get().await?;
            self.controller_sender
                .send(ClientToModuleEvent::DeviceChanged {
                    number,
                    r#type: r#type.clone(),
                    new_state,
                })?;
        }

        Ok(())
    }

    async fn dbus_connection(&self) -> Result<Connection> {
        let dbus_connection_guard = self.dbus_connection.read().await;
        if let Some(dbus_connection) = &*dbus_connection_guard {
            Ok(dbus_connection.clone())
        } else {
            // Yes it's a bit awkward to first obtain a read lock and then a write lock but it
            //  needs to happen only once, and after that all read lock acquisitions will be
            //  instant
            drop(dbus_connection_guard);
            let dbus_connection = Connection::system().await?;
            *self.dbus_connection.write().await = Some(dbus_connection.clone());
            Ok(dbus_connection)
        }
    }
}

pub fn create_client() -> ClientResult<Client> {
    let client = Arc::new(Client::new());
    client.run()?;
    Ok(client)
}

fn get_number_from_dbus_path(path: &ObjectPath) -> u32 {
    let (_, number_str) = path
        .rsplit_once('/')
        .expect("Path must have at least two segments to contain an object number");
    number_str
        .parse()
        .expect("Last segment was not a positive integer")
}

register_fallible_client!(Client, network_manager);
