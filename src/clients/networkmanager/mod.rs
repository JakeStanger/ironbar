use color_eyre::Result;
use color_eyre::eyre::Ok;
use futures_lite::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use zbus::Connection;
use zbus::zvariant::{ObjectPath, Str};

use crate::clients::ClientResult;
use crate::clients::networkmanager::dbus::{DbusProxy, DeviceDbusProxy};
use crate::clients::networkmanager::event::Event;
use crate::{register_fallible_client, spawn};

pub mod dbus;
pub mod event;

#[derive(Debug)]
pub struct Client {
    tx: broadcast::Sender<Event>,
}

impl Client {
    fn new() -> Result<Client> {
        let (tx, _) = broadcast::channel(64);
        Ok(Client { tx })
    }

    fn run(&self) -> Result<()> {
        let tx = self.tx.clone();
        spawn(async move {
            let dbus_connection = Connection::system().await?;
            let root = DbusProxy::new(&dbus_connection).await?;

            let mut devices = HashSet::new();

            let mut devices_changes = root.receive_all_devices_changed().await;
            while let Some(devices_change) = devices_changes.next().await {
                // The new list of devices from dbus, not to be confused with the added devices below
                let new_devices = HashSet::from_iter(devices_change.get().await?);

                let added_devices = new_devices.difference(&devices);
                for added_device in added_devices {
                    spawn(watch_device(added_device.to_owned(), tx.clone()));
                }

                let removed_devices = devices.difference(&new_devices);
                // TODO: Cook up some way to notify closures for removed devices to exit

                devices = new_devices;
            }

            Ok(())
        });

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

pub fn create_client() -> ClientResult<Client> {
    let client = Arc::new(Client::new()?);
    client.run()?;
    Ok(client)
}

async fn watch_device(device_path: ObjectPath<'_>, tx: broadcast::Sender<Event>) -> Result<()> {
    let dbus_connection = Connection::system().await?;
    let device = DeviceDbusProxy::new(&dbus_connection, device_path.to_owned()).await?;

    let interface = device.interface().await?;
    tx.send(Event::DeviceAdded {
        interface: interface.to_string(),
    })?;

    spawn(watch_device_state(
        device_path.to_owned(),
        interface.to_owned(),
        tx.clone(),
    ));

    Ok(())
}

async fn watch_device_state(
    device_path: ObjectPath<'_>,
    interface: Str<'_>,
    tx: broadcast::Sender<Event>,
) -> Result<()> {
    let dbus_connection = Connection::system().await?;
    let device = DeviceDbusProxy::new(&dbus_connection, &device_path).await?;
    let r#type = device.device_type().await?;

    // Send an event communicating the initial state
    let state = device.state().await?;
    tx.send(Event::DeviceStateChanged {
        interface: interface.to_string(),
        r#type: r#type.clone(),
        state,
    })?;

    let mut state_changes = device.receive_state_changed().await;
    while let Some(state_change) = state_changes.next().await {
        let state = state_change.get().await?;
        tx.send(Event::DeviceStateChanged {
            interface: interface.to_string(),
            r#type: r#type.clone(),
            state,
        })?;
    }

    Ok(())
}

register_fallible_client!(Client, network_manager);
