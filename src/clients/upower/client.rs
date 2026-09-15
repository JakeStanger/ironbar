use super::dbus::UPowerProxy;
use super::{BatteryState, State};
use crate::channels::SyncSenderExt;
use crate::clients::ClientResult;
use crate::{await_sync, spawn};
use futures_lite::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use zbus::Result;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;

/// Reads the kernel charge cap from `/sys/class/power_supply/*/charge_control_end_threshold`.
/// Returns `None` when no battery exposes the file or the value is 0/100 (no effective cap).
fn read_charge_cap_pct() -> Option<f64> {
    std::fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .find_map(|e| {
            std::fs::read_to_string(e.path().join("charge_control_end_threshold"))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
        })
        .filter(|&t| (1..=99).contains(&t))
        .map(f64::from)
}

/// Scales UPower `time_to_full` (seconds to 100%) to time-to-cap when a kernel
/// charge cap is set. Returns the input unchanged when no cap applies or the
/// battery is already at/past the cap.
fn scale_ttf(ttf: i64, battery_pct: f64) -> i64 {
    let Some(cap_pct) = read_charge_cap_pct().filter(|&cap| battery_pct < cap) else {
        return ttf;
    };
    let scale = (cap_pct - battery_pct) / (100.0 - battery_pct);
    (ttf as f64 * scale).round() as i64
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
                            "EnergyRate" => {
                                state.energy_rate = value.downcast::<f64>().unwrap_or_default();
                            }
                            _ => {}
                        }
                    }

                    let mut emit = state.clone();
                    emit.time_to_full = scale_ttf(emit.time_to_full, emit.percentage);
                    tx.send_expect(emit);
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
        let mut state: State = self
            .proxy
            .get_all(self.interface_name.clone())
            .await?
            .try_into()?;
        state.time_to_full = scale_ttf(state.time_to_full, state.percentage);
        Ok(state)
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
