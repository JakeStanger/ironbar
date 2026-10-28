use super::{BatteryLevel, BatteryState, BatteryType, State};
use crate::channels::SyncSenderExt;
use crate::ironvar::NamespaceTrait;
use crate::spawn;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

fn base_state() -> State {
    State {
        percentage: 100.0,
        icon_name: "battery-full".to_string(),
        state: BatteryState::Discharging,
        time_to_full: 0,
        time_to_empty: 20 * 60,
        level: BatteryLevel::Full,
        battery_type: BatteryType::Battery,
    }
}

#[derive(Debug)]
pub enum Client {
    Stopped {
        tx: broadcast::Sender<State>,
        _rx: broadcast::Receiver<State>,
    },
    Started {
        tx: broadcast::Sender<State>,
        _rx: broadcast::Receiver<State>,
        handle: JoinHandle<()>,
    },
}

impl Client {
    pub(in crate::clients) fn new() -> Self {
        let (tx, rx) = broadcast::channel(16);
        Self::Stopped { tx, _rx: rx }
    }

    fn tx(&self) -> &broadcast::Sender<State> {
        match self {
            Self::Stopped { tx, .. } => tx,
            Self::Started { tx, .. } => tx,
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
        let Self::Stopped { tx, _rx } = self else {
            return Ok(self);
        };

        let handle = spawn({
            let tx = tx.clone();
            async move {
                let mut state = base_state();

                loop {
                    state.state = BatteryState::Discharging;

                    for i in (0..=100).rev() {
                        state.percentage = i as f64;
                        state.icon_name = match i {
                            0 => "battery-empty",
                            1..20 => "battery-caution",
                            20..50 => "battery-low",
                            50..75 => "battery-good",
                            75..100 => "battery-full",
                            100 => "battery-fully-charged",
                            _ => "",
                        }
                        .to_string();

                        state.level = match i {
                            0..10 => BatteryLevel::Critical,
                            10..33 => BatteryLevel::Low,
                            33..67 => BatteryLevel::Normal,
                            67..100 => BatteryLevel::Full,
                            100 => BatteryLevel::Full,
                            _ => BatteryLevel::Unknown,
                        };

                        state.time_to_empty = i * 60;

                        tx.send_expect(state.clone());

                        tokio::time::sleep(Duration::from_millis(20_000 / 100)).await;
                    }

                    state.state = BatteryState::Charging;

                    for i in 0..=100 {
                        state.percentage = i as f64;
                        state.icon_name = match i {
                            0 => "battery-empty-charging",
                            1..20 => "battery-caution-charging",
                            20..50 => "battery-low-charging",
                            50..75 => "battery-good-charging",
                            75..=100 => "battery-full-charging",
                            _ => "",
                        }
                        .to_string();
                        state.time_to_full = (100 - i) * 60;

                        tx.send_expect(state.clone());

                        tokio::time::sleep(Duration::from_millis(20_000 / 100)).await;
                    }
                }
            }
        });

        Ok(Self::Started { tx, _rx, handle })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        let Self::Started { tx, _rx, handle } = self else {
            return Ok(self);
        };

        handle.abort();

        Ok(Self::Stopped { tx, _rx })
    }

    async fn state(&self) -> crate::clients::Result<Self::State, Self::Error> {
        if !self.is_started() {
            return Err(crate::clients::Error::InvalidState);
        }

        Ok(base_state())
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

#[cfg(any(feature = "ipc", feature = "cairo"))]
impl crate::ironvar::Namespace for Client {
    fn get(&self, _: &str) -> Option<String> {
        None
    }

    fn list(&self) -> Vec<String> {
        vec![]
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _: &str) -> Option<NamespaceTrait> {
        None
    }
}
