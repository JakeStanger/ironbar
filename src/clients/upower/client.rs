use super::dbus::UPowerProxy;
use super::{BatteryState, State};
use crate::channels::{BroadcastChannel, SyncSenderExt};
use crate::{await_sync, spawn, stop_basic};
use futures_lite::StreamExt;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::debug;
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
pub enum Client {
    Stopped(BroadcastChannel<State>),
    Started {
        channel: BroadcastChannel<State>,
        proxy: PropertiesProxy<'static>,
        handle: JoinHandle<()>,
    },
}

impl Client {
    pub(in crate::clients) fn new() -> Self {
        Self::Stopped(BroadcastChannel::new(16))
    }

    fn interface_name() -> InterfaceName<'static> {
        InterfaceName::from_static_str("org.freedesktop.UPower.Device")
            .expect("failed to create zbus InterfaceName")
    }

    fn proxy(&self) -> crate::clients::Result<&PropertiesProxy<'static>, zbus::Error> {
        match self {
            Self::Stopped { .. } => Err(crate::clients::Error::InvalidState),
            Self::Started { proxy, .. } => Ok(proxy),
        }
    }

    fn tx(&self) -> &broadcast::Sender<State> {
        match self {
            Self::Stopped(channel) => channel,
            Self::Started { channel, .. } => channel,
        }
    }
}

impl crate::clients::Client for Client {
    type State = State;
    type Event = State;
    type Error = zbus::Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> Result<Self, Self::Error> {
        let Self::Stopped(channel) = self else {
            return Ok(self);
        };

        let dbus = Box::pin(zbus::Connection::system()).await?;

        let device_proxy = UPowerProxy::new(&dbus).await?;

        let display_device = device_proxy.get_display_device().await?;

        let path = display_device.inner().path();

        let proxy = PropertiesProxy::builder(&dbus)
            .destination("org.freedesktop.UPower")
            .path(path)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;

        let interface_name = Self::interface_name();

        let mut stream = proxy.receive_properties_changed().await?;
        let mut state: State = proxy.get_all(interface_name.clone()).await?.try_into()?;

        let handle = spawn({
            let tx = channel.clone();

            async move {
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

                    let mut emit = state.clone();
                    emit.time_to_full = scale_ttf(emit.time_to_full, emit.percentage);
                    tx.send_expect(emit);
                }

                debug!("upower properties stream ended");
            }
        });

        Ok(Self::Started {
            channel,
            proxy,
            handle,
        })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        stop_basic!(self)
    }

    async fn state(&self) -> crate::clients::Result<Self::State, Self::Error> {
        let mut state: State = self
            .proxy()?
            .get_all(Self::interface_name())
            .await
            .map_err(zbus::Error::from)?
            .try_into()?;
        state.time_to_full = scale_ttf(state.time_to_full, state.percentage);
        Ok(state)
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

#[cfg(any(feature = "ipc", feature = "cairo"))]
impl crate::ironvar::Namespace for Client {
    fn get(&self, key: &str) -> Option<String> {
        let proxy = self.proxy().ok()?;
        let value = await_sync(async { proxy.get(Self::interface_name(), key).await }).ok();
        value.map(|v| v.to_string())
    }

    fn list(&self) -> Vec<String> {
        self.get_all().keys().map(ToString::to_string).collect()
    }

    fn get_all(&self) -> HashMap<Box<str>, String> {
        let Ok(proxy) = self.proxy() else {
            return HashMap::new();
        };

        let properties = await_sync(async { proxy.get_all(Self::interface_name()).await })
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
