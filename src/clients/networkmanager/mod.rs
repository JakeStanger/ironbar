use color_eyre::Result;
use color_eyre::eyre::Ok;
use futures_lite::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
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
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
    sender: broadcast::Sender<ModuleToClientEvent>,
}

impl Client {
    fn new() -> Result<Client> {
        let (controller_sender, _) = broadcast::channel(64);
        let (sender, _) = broadcast::channel(8);
        Ok(Client {
            controller_sender,
            sender,
        })
    }

    fn run(&self) -> Result<()> {
        debug!("Client running");

        let devices = Arc::new(RwLock::new(HashSet::<ObjectPath>::new()));

        spawn(watch_devices_list(
            devices.clone(),
            self.controller_sender.clone(),
        ));

        let receiver = self.sender.subscribe();
        spawn(handle_received_events(
            receiver,
            devices.clone(),
            self.controller_sender.clone(),
        ));

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ClientToModuleEvent> {
        self.controller_sender.subscribe()
    }

    pub fn get_sender(&self) -> broadcast::Sender<ModuleToClientEvent> {
        self.sender.clone()
    }
}

pub fn create_client() -> ClientResult<Client> {
    let client = Arc::new(Client::new()?);
    client.run()?;
    Ok(client)
}

async fn watch_devices_list(
    devices: Arc<RwLock<HashSet<ObjectPath<'_>>>>,
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
) -> Result<()> {
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

        // Atomic read-then-write of `devices`
        let mut devices_locked = devices.write().await;
        let devices_snapshot = devices_locked.clone();
        (*devices_locked).clone_from(&new_devices);
        drop(devices_locked);

        let added_devices = new_devices.difference(&devices_snapshot);
        for added_device in added_devices {
            spawn(watch_device(
                added_device.to_owned(),
                controller_sender.clone(),
            ));
        }

        let _removed_devices = devices_snapshot.difference(&new_devices);
        // TODO: Store join handles for watchers and abort them when their device is removed
        // TODO: Inform module of removed devices
    }

    Ok(())
}

async fn handle_received_events(
    mut receiver: broadcast::Receiver<ModuleToClientEvent>,
    devices: Arc<RwLock<HashSet<ObjectPath<'_>>>>,
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
) -> Result<()> {
    while let Result::Ok(event) = receiver.recv().await {
        match event {
            ModuleToClientEvent::NewController => {
                debug!("Client received NewController event");

                let dbus_connection = Connection::system().await?;

                // We create a local clone here to avoid holding the lock for too long
                let devices_snapshot = devices.read().await.clone();

                for device_path in devices_snapshot {
                    let device = DeviceDbusProxy::new(&dbus_connection, device_path).await?;

                    let interface = device.interface().await?.to_string();
                    let r#type = device.device_type().await?;
                    let new_state = device.state().await?;
                    controller_sender.send(ClientToModuleEvent::DeviceChanged {
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

async fn watch_device(
    path: ObjectPath<'_>,
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
) -> Result<()> {
    let dbus_connection = Connection::system().await?;
    let device = DeviceDbusProxy::new(&dbus_connection, path.to_owned()).await?;

    spawn(watch_device_state(device, controller_sender));

    Ok(())
}

async fn watch_device_state(
    device: DeviceDbusProxy<'_>,
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
) -> Result<()> {
    let path = device.inner().path();

    debug!("D-Bus device state watcher for {} starting", path);

    let interface = device.interface().await?;
    let r#type = device.device_type().await?;

    // Send an event communicating the initial state
    let new_state = device.state().await?;
    controller_sender.send(ClientToModuleEvent::DeviceChanged {
        interface: interface.to_string(),
        r#type: r#type.clone(),
        new_state,
    })?;

    let mut state_changes = device.receive_state_changed().await;
    while let Some(state_change) = state_changes.next().await {
        let new_state = state_change.get().await?;
        controller_sender.send(ClientToModuleEvent::DeviceChanged {
            interface: interface.to_string(),
            r#type: r#type.clone(),
            new_state,
        })?;
    }

    debug!("D-Bus device state watcher for {} ended", path);

    Ok(())
}

register_fallible_client!(Client, network_manager);
