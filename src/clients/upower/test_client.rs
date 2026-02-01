use super::{BatteryState, State};
use crate::channels::SyncSenderExt;
use crate::clients::ClientResult;
use crate::ironvar::NamespaceTrait;
use crate::spawn;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use zbus::Result;

fn base_state() -> State {
    State {
        percentage: 100.0,
        icon_name: "battery-full".to_string(),
        state: BatteryState::Discharging,
        time_to_full: 0,
        time_to_empty: 20 * 60,
    }
}

#[derive(Debug)]
pub struct Client {
    tx: broadcast::Sender<State>,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let (tx, rx) = broadcast::channel(16);
        std::mem::forget(rx);

        spawn({
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

        Ok(Arc::new(Self { tx }))
    }

    pub async fn state(&self) -> Result<State> {
        Ok(base_state())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<State> {
        self.tx.subscribe()
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
