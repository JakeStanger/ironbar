use color_eyre::Result;
use color_eyre::eyre::Error;
use futures_lite::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::join;
use tokio::sync::{RwLock, broadcast};
use tokio::time::sleep;
use tokio_stream::StreamMap;
use zbus::Connection;
use zbus::proxy::PropertyStream;
use zbus::zvariant::ObjectPath;

use crate::clients::networkmanager::dbus::{DbusProxy, Device, DeviceDbusProxy, DeviceState};
use crate::clients::networkmanager::event::Event;
use crate::{register_fallible_client, spawn};

mod dbus;
pub mod event;

#[derive(Debug)]
pub struct Client {
    tx: broadcast::Sender<Event>,
}

impl Client {
    async fn new() -> Result<Client> {
        let (tx, _) = broadcast::channel(64);
        Ok(Client { tx })
    }

    async fn run(&self) -> Result<()> {
        let dbus_connection = Connection::system().await?;
        let root_object = DbusProxy::new(&dbus_connection).await?;

        let device_state_changes =
            RwLock::new(StreamMap::<Device, PropertyStream<DeviceState>>::new());

        let _ = join!(
            // Handles the addition and removal of device objects
            async {
                let mut devices_changes = root_object.receive_devices_changed().await;
                while let Some(change) = devices_changes.next().await {
                    println!("here?");

                    let devices = HashSet::from_iter(
                        device_state_changes
                            .read()
                            .await
                            .keys()
                            .map(|device| &device.object_path)
                            .cloned(),
                    );

                    // The new list of devices from dbus, not to be confused with the added devices below
                    let new_devices_vec = change.get().await?;
                    let new_devices = HashSet::<ObjectPath>::from_iter(new_devices_vec);
                    println!("Existing devices: {:?}", devices);
                    println!("New devices: {:?}", new_devices);

                    let added_devices = new_devices.difference(&devices);
                    println!("Added devices: {:?}", added_devices);
                    for added_device in added_devices {
                        let device_proxy =
                            DeviceDbusProxy::new(&dbus_connection, added_device).await?;
                        let device_type = device_proxy.device_type().await?;
                        let device_state_stream = device_proxy.receive_state_changed().await;
                        device_state_changes.write().await.insert(
                            Device {
                                object_path: added_device.clone(),
                                type_: device_type.clone(), // TODO: Remove clone when removing println below
                            },
                            device_state_stream,
                        );
                        println!("Device added: {} type {:?}", added_device, device_type);
                    }

                    let removed_devices = devices.difference(&new_devices);
                    println!("Removed devices: {:?}", removed_devices);
                    for removed_device in removed_devices {
                        let device_proxy =
                            DeviceDbusProxy::new(&dbus_connection, removed_device).await?;
                        let device_type = device_proxy.device_type().await?;
                        device_state_changes.write().await.remove(&Device {
                            object_path: removed_device.clone(),
                            type_: device_type.clone(), // TODO: Remove clone when removing println below
                        });
                        println!("Device removed: {} type {:?}", removed_device, device_type);
                    }
                }
                Ok::<(), Error>(())
            },
            // Handles changes to device properties
            async {
                sleep(Duration::from_secs(5)).await;

                /*
                Okay so this causes a deadlock, and we should rewrite all of this with spawn() anyway cause join!() is not multithreaded apparently.
                In order to not leak memory we could have closures for objects that don't exist anymore check this manually and return.
                */
                while let Some((device, property)) = device_state_changes.write().await.next().await
                {
                    let property = property.get().await?;
                    println!(
                        "Device state changed: {} to {:?}",
                        device.object_path, property
                    );
                }

                println!("Prop loop ended");

                Ok::<(), Error>(())
            },
        );

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

pub async fn create_client() -> Result<Arc<Client>> {
    // TODO: Use spawn here after all, otherwise we block on creation

    let client = Arc::new(Client::new().await?);
    client.run().await?;
    Ok(client)
}

register_fallible_client!(Client, network_manager);
