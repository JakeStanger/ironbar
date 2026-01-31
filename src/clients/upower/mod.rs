mod dbus;

use super::ClientResult;
use crate::channels::SyncSenderExt;
use crate::{await_sync, register_fallible_client, spawn};
pub use dbus::BatteryState;
use dbus::UPowerProxy;
use futures_lite::StreamExt;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use tokio::sync::broadcast;
use zbus::Result;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedValue;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub percentage: f64,
    pub icon_name: String,
    pub state: BatteryState,
    pub time_to_full: i64,
    pub time_to_empty: i64,
}

impl TryFrom<HashMap<String, OwnedValue>> for State {
    type Error = zbus::zvariant::Error;

    fn try_from(properties: HashMap<String, OwnedValue>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            percentage: properties["Percentage"].downcast_ref::<f64>()?,
            icon_name: properties["IconName"].downcast_ref::<&str>()?.to_string(),
            state: properties["State"].downcast_ref::<BatteryState>()?,
            time_to_full: properties["TimeToFull"].downcast_ref::<i64>()?,
            time_to_empty: properties["TimeToEmpty"].downcast_ref::<i64>()?,
        })
    }
}

impl Display for BatteryState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BatteryState::Unknown => "Unknown",
                BatteryState::Charging => "Charging",
                BatteryState::Discharging => "Discharging",
                BatteryState::Empty => "Empty",
                BatteryState::FullyCharged => "Fully charged",
                BatteryState::PendingCharge => "Pending charge",
                BatteryState::PendingDischarge => "Pending discharge",
            }
        )
    }
}

#[derive(Debug)]
pub struct Client {
    proxy: PropertiesProxy<'static>,
    interface_name: InterfaceName<'static>,

    tx: broadcast::Sender<State>,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let dbus = Box::pin(zbus::Connection::system()).await?;

        let device_proxy = UPowerProxy::new(&dbus).await?;

        let display_device = device_proxy.get_display_device().await?;

        let path = display_device.inner().path();

        let proxy = PropertiesProxy::builder(&dbus)
            .destination("org.freedesktop.UPower")
            .expect("failed to set proxy destination address")
            .path(path)
            .expect("failed to set proxy path")
            .cache_properties(CacheProperties::No)
            .build()
            .await?;

        let interface_name = InterfaceName::from_static_str("org.freedesktop.UPower.Device")
            .expect("failed to create zbus InterfaceName");

        let (tx, rx) = broadcast::channel(16);
        std::mem::forget(rx);

        spawn({
            let tx = tx.clone();
            let proxy = proxy.clone();
            let interface_name = interface_name.clone();

            let mut stream = proxy.receive_properties_changed().await?;

            async move {
                let mut state: State = proxy.get_all(interface_name.clone()).await?.try_into()?;

                while let Some(ev) = stream.next().await {
                    let args = ev.args().expect("Invalid signal arguments");
                    if args.interface_name != interface_name {
                        continue;
                    }

                    for (key, value) in args.changed_properties {
                        match key {
                            "Percentage" => {
                                state.percentage = value.downcast::<f64>().unwrap_or_default();
                            }
                            "IconName" => {
                                state.icon_name = value.downcast::<String>().unwrap_or_default();
                            }
                            "State" => {
                                state.state =
                                    value.downcast_ref::<BatteryState>().unwrap_or_default();
                            }
                            "TimeToFull" => {
                                state.time_to_full = value.downcast::<i64>().unwrap_or_default();
                            }
                            "TimeToEmpty" => {
                                state.time_to_empty = value.downcast::<i64>().unwrap_or_default();
                            }
                            _ => {}
                        }
                    }

                    tx.send_expect(state.clone());
                }

                Ok::<_, zbus::Error>(())
            }
        });

        Ok(Arc::new(Self {
            proxy,
            interface_name,
            tx,
        }))
    }

    pub async fn state(&self) -> Result<State> {
        Ok(self
            .proxy
            .get_all(self.interface_name.clone())
            .await?
            .try_into()?)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<State> {
        self.tx.subscribe()
    }
}

#[cfg(any(feature = "ipc", feature = "cairo"))]
impl crate::ironvar::Namespace for Client {
    fn get(&self, key: &str) -> Option<String> {
        let value =
            await_sync(async { self.proxy.get(self.interface_name.clone(), key).await }).ok();
        value.map(|v| v.to_string())
    }

    fn list(&self) -> Vec<String> {
        self.get_all().keys().map(ToString::to_string).collect()
    }

    fn get_all(&self) -> HashMap<Box<str>, String> {
        let properties =
            await_sync(async { self.proxy.get_all(self.interface_name.clone()).await })
                .ok()
                .unwrap_or_default();

        properties
            .into_iter()
            .map(|(k, v)| (k.into(), v.to_string()))
            .collect()
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        None
    }
}

register_fallible_client!(Client, upower);
