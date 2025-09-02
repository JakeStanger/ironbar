use color_eyre::Result;
use color_eyre::eyre::Ok;
use futures_lite::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use zbus::Connection;
use zbus::zvariant::{ObjectPath, Str};

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
        let devices: &'static _ = Box::leak(Box::new(RwLock::new(HashSet::<ObjectPath>::new())));

        {
            let controller_sender = self.controller_sender.clone();
            spawn(async move {
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

                    // We create a local clone here to avoid holding the lock for too long
                    let devices_snapshot = devices.read().await.clone();

                    let added_devices = new_devices.difference(&devices_snapshot);
                    for added_device in added_devices {
                        spawn(watch_device(
                            added_device.to_owned(),
                            controller_sender.clone(),
                        ));
                    }

                    let _removed_devices = devices_snapshot.difference(&new_devices);
                    // TODO: Cook up some way to notify closures for removed devices to exit

                    *devices.write().await = new_devices;
                }

                Ok(())
            });
        }

        {
            let controller_sender = self.controller_sender.clone();
            let mut receiver = self.sender.subscribe();
            spawn(async move {
                while let Result::Ok(event) = receiver.recv().await {
                    match event {
                        ModuleToClientEvent::NewController => {
                            // We create a local clone here to avoid holding the lock for too long
                            let devices_snapshot = devices.read().await.clone();

                            for device_path in devices_snapshot {
                                let dbus_connection = Connection::system().await?;
                                let device =
                                    DeviceDbusProxy::new(&dbus_connection, device_path).await?;

                                // TODO: Create DeviceDbusProxy -> DeviceStateChanged function and use it in the watcher as well
                                let interface = device.interface().await?.to_string();
                                let r#type = device.device_type().await?;
                                let state = device.state().await?;
                                controller_sender.send(
                                    ClientToModuleEvent::DeviceStateChanged {
                                        interface,
                                        r#type,
                                        state,
                                    },
                                )?;
                            }
                        }
                    }
                }

                Ok(())
            });
        }

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

async fn watch_device(
    device_path: ObjectPath<'_>,
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
) -> Result<()> {
    let dbus_connection = Connection::system().await?;
    let device = DeviceDbusProxy::new(&dbus_connection, device_path.to_owned()).await?;

    let interface = device.interface().await?;

    spawn(watch_device_state(
        device_path.to_owned(),
        interface.to_owned(),
        controller_sender.clone(),
    ));

    Ok(())
}

async fn watch_device_state(
    device_path: ObjectPath<'_>,
    interface: Str<'_>,
    controller_sender: broadcast::Sender<ClientToModuleEvent>,
) -> Result<()> {
    let dbus_connection = Connection::system().await?;
    let device = DeviceDbusProxy::new(&dbus_connection, &device_path).await?;
    let r#type = device.device_type().await?;

    // Send an event communicating the initial state
    let state = device.state().await?;
    controller_sender.send(ClientToModuleEvent::DeviceStateChanged {
        interface: interface.to_string(),
        r#type: r#type.clone(),
        state,
    })?;

    let mut state_changes = device.receive_state_changed().await;
    while let Some(state_change) = state_changes.next().await {
        let state = state_change.get().await?;
        controller_sender.send(ClientToModuleEvent::DeviceStateChanged {
            interface: interface.to_string(),
            r#type: r#type.clone(),
            state,
        })?;
    }

    Ok(())
}

register_fallible_client!(Client, network_manager);
