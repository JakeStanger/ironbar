use std::time::Duration;

use crate::{Ironbar, error, register_fallible_client, spawn};
use color_eyre::Result;
use tokio::{sync::watch, task::JoinSet};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothState {
    Enabling,
    Enabled {
        /// Sorted by `address`
        devices: Vec<BluetoothDevice>,
    },
    Disabled,
    Disabling,
    NotFound,
}

impl BluetoothState {
    pub fn is_enabled(&self) -> bool {
        matches!(self, BluetoothState::Enabled { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothDeviceStatus {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    /// Unique device identifier.
    pub address: bluer::Address,

    /// Remote device connection status
    pub status: BluetoothDeviceStatus,

    /// Device alias (if set) or its name.
    pub alias: String,

    /// Proposed icon name according to the freedesktop.org naming specification.
    /// Received from `bluer` as is.
    pub icon: Option<String>,

    /// The battery percentage.
    pub battery_percent: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct Client {
    session: bluer::Session,

    tx: watch::Sender<BluetoothState>,
    _rx: watch::Receiver<BluetoothState>,
}

impl Client {
    pub(crate) async fn new() -> Result<Self> {
        let (tx, _rx) = watch::channel(BluetoothState::NotFound);
        let session = bluer::Session::new().await?;
        {
            let tx = tx.clone();
            let session = session.clone();
            spawn(async move {
                debug!("Starting bluetooth session");

                // Ideally, this loop should be replaced by a subscription to
                // bluetooth adapter events (see `bluer::Adapter::events`).
                // But unfortunately is does now work very reliable, sometimes
                // missing important events. So instead we just get a full state
                // every 5 seconds.
                //
                // This does not affect responsiveness, as user actions force
                // an immediate state update.
                loop {
                    match Self::get_state(&session).await {
                        Ok(mut new_state) => {
                            debug!("New bluetooth state: {:?}", new_state);

                            if 0 == tx.receiver_count() {
                                break;
                            }

                            tx.send_modify(|old_state| {
                                Self::patch_new_state(&mut new_state, old_state);
                                *old_state = new_state;
                            });
                        }
                        Err(err) => {
                            error!("Bluetooth error: {}", err);
                        }
                    }

                    tokio::time::sleep(Duration::from_secs(5)).await;
                }

                debug!("Stopping bluetooth session");
            });
        }

        Ok(Self { session, tx, _rx })
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<BluetoothState> {
        self.tx.subscribe()
    }

    pub(crate) fn enable(&self) {
        debug!("Enabling");

        let session = self.session.clone();
        let tx = self.tx.clone();
        spawn(async move {
            if !tx.send_if_modified(|state| {
                if matches!(state, BluetoothState::Disabled) {
                    *state = BluetoothState::Enabling;
                    true
                } else {
                    false
                }
            }) {
                debug!("Already enabled");
                return;
            }

            let new_state = if let Err(err) = Self::set_powered(session, true).await {
                error!("Failed to enable: {}", err);
                BluetoothState::Disabled
            } else {
                debug!("Enabled");
                BluetoothState::Enabled {
                    devices: Vec::new(),
                }
            };

            tx.send_if_modified(|state| {
                if matches!(state, BluetoothState::Enabling) {
                    *state = new_state;
                    true
                } else {
                    false
                }
            });
        });
    }

    pub(crate) fn disable(&self) {
        debug!("Disabling");

        let session = self.session.clone();
        let tx = self.tx.clone();
        spawn(async move {
            if !tx.send_if_modified(|state| {
                if matches!(state, BluetoothState::Enabled { .. }) {
                    *state = BluetoothState::Disabling;
                    true
                } else {
                    false
                }
            }) {
                debug!("Already disabled");
                return;
            }

            let new_state = if let Err(err) = Self::set_powered(session, false).await {
                error!("Failed to disable: {}", err);
                BluetoothState::Enabled {
                    devices: Vec::new(),
                }
            } else {
                debug!("Disabled");
                BluetoothState::Disabled
            };

            tx.send_if_modified(|state| {
                if matches!(state, BluetoothState::Disabling) {
                    *state = new_state;
                    true
                } else {
                    false
                }
            });
        });
    }

    pub(crate) fn connect(&self, address: bluer::Address) {
        debug!("Connect {}", address);

        let session = self.session.clone();
        let tx = self.tx.clone();
        spawn(async move {
            if !tx.send_if_modified(|state| {
                Self::try_replace_device_status(
                    state,
                    address,
                    BluetoothDeviceStatus::Disconnected,
                    BluetoothDeviceStatus::Connecting,
                )
            }) {
                return;
            }

            let new_status = if let Err(err) = Self::connect_device(session, address).await {
                error!("Connection error: {}", err);
                BluetoothDeviceStatus::Disconnected
            } else {
                debug!("Connect finished {}", address);
                BluetoothDeviceStatus::Connected
            };

            tx.send_if_modified(|state| {
                Self::try_replace_device_status(
                    state,
                    address,
                    BluetoothDeviceStatus::Connecting,
                    new_status,
                )
            });
        });
    }

    pub(crate) fn disconnect(&self, address: bluer::Address) {
        debug!("Disconnect {}", address);

        let session = self.session.clone();
        let tx = self.tx.clone();
        spawn(async move {
            if !tx.send_if_modified(|state| {
                Self::try_replace_device_status(
                    state,
                    address,
                    BluetoothDeviceStatus::Connected,
                    BluetoothDeviceStatus::Disconnecting,
                )
            }) {
                return;
            }

            let new_status = if let Err(err) = Self::disconnect_device(session, address).await {
                error!("Disconnect error: {}", err);
                BluetoothDeviceStatus::Connected
            } else {
                debug!("Disconnect finished {}", address);
                BluetoothDeviceStatus::Disconnected
            };

            tx.send_if_modified(|state| {
                Self::try_replace_device_status(
                    state,
                    address,
                    BluetoothDeviceStatus::Disconnecting,
                    new_status,
                )
            });
        });
    }

    async fn set_powered(session: bluer::Session, val: bool) -> Result<()> {
        let adapter = session.default_adapter().await?;
        adapter.set_powered(val).await?;
        Ok(())
    }

    async fn connect_device(session: bluer::Session, address: bluer::Address) -> Result<()> {
        let adapter = session.default_adapter().await?;
        let device = adapter.device(address)?;
        device.connect().await?;

        Ok(())
    }

    async fn disconnect_device(session: bluer::Session, address: bluer::Address) -> Result<()> {
        let adapter = session.default_adapter().await?;
        let device = adapter.device(address)?;
        device.disconnect().await?;

        Ok(())
    }

    async fn get_device(
        adapter: bluer::Adapter,
        address: bluer::Address,
    ) -> Result<BluetoothDevice> {
        let device = adapter.device(address)?;

        // Should be patched after
        let status = if device.is_connected().await? {
            BluetoothDeviceStatus::Connected
        } else {
            BluetoothDeviceStatus::Disconnected
        };

        Ok(BluetoothDevice {
            address,
            status,
            alias: device.alias().await?,
            icon: device.icon().await?,
            battery_percent: device.battery_percentage().await?,
        })
    }

    async fn get_state(session: &bluer::Session) -> Result<BluetoothState> {
        let state = match session.default_adapter().await {
            Ok(adapter) => {
                if adapter.is_powered().await? {
                    let addrs = adapter.device_addresses().await?;

                    let mut joinset = JoinSet::new();

                    for addr in addrs {
                        let adapter = adapter.clone();
                        joinset.spawn_on(
                            async move { Self::get_device(adapter, addr).await },
                            Ironbar::runtime().handle(),
                        );
                    }

                    let mut devices = joinset
                        .join_all()
                        .await
                        .into_iter()
                        .collect::<Result<Vec<_>>>()?;

                    devices.sort_by_key(|d| d.address);

                    BluetoothState::Enabled { devices }
                } else {
                    BluetoothState::Disabled
                }
            }
            Err(bluer::Error {
                kind: bluer::ErrorKind::NotFound,
                ..
            }) => BluetoothState::NotFound,
            Err(err) => return Err(err.into()),
        };

        Ok(state)
    }

    /// Preserves changes that were made in `old_state` by `connect`, `disconnect` and `set_enabled` methods.
    /// These changes are transient states of adapter and devices like `Connecting`, `Enabling`, etc
    fn patch_new_state(new_state: &mut BluetoothState, old_state: &BluetoothState) {
        match (new_state, old_state) {
            (new_state @ BluetoothState::Enabled { .. }, BluetoothState::Disabling) => {
                *new_state = BluetoothState::Disabling;
            }
            (new_state @ BluetoothState::Disabled, BluetoothState::Enabling) => {
                *new_state = BluetoothState::Enabling;
            }
            (
                BluetoothState::Enabled {
                    devices: new_devices,
                },
                BluetoothState::Enabled {
                    devices: old_devices,
                },
            ) => {
                for new_device in new_devices {
                    if let Ok(idx) =
                        old_devices.binary_search_by_key(&new_device.address, |d| d.address)
                    {
                        let old_device = &old_devices[idx];

                        match (new_device.status, old_device.status) {
                            (
                                BluetoothDeviceStatus::Connected,
                                BluetoothDeviceStatus::Disconnecting,
                            ) => {
                                new_device.status = BluetoothDeviceStatus::Disconnecting;
                            }
                            (
                                BluetoothDeviceStatus::Disconnected,
                                BluetoothDeviceStatus::Connecting,
                            ) => {
                                new_device.status = BluetoothDeviceStatus::Connecting;
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => (),
        }
    }

    /// Tryies to change device status from `old_status` to `new_status`.
    /// Returns true on success
    fn try_replace_device_status(
        state: &mut BluetoothState,
        address: bluer::Address,
        old_status: BluetoothDeviceStatus,
        new_status: BluetoothDeviceStatus,
    ) -> bool {
        match state {
            BluetoothState::Enabled { devices } => {
                if let Ok(idx) = devices.binary_search_by_key(&address, |d| d.address) {
                    let device = &mut devices[idx];
                    if device.status == old_status {
                        device.status = new_status;
                        return true;
                    }

                    debug!(
                        "Failed change device status from {:?} to {:?} of device {:?} because device is in {:?} status",
                        old_status, new_status, address, device.status
                    );
                } else {
                    error!(
                        "Failed change status to {:?} of unknown device {:?}",
                        new_status, address
                    );
                }
            }
            _ => {
                error!(
                    "Failed to change status to {:?} while in {:?} of device {:?}",
                    new_status, address, state
                );
            }
        }

        false
    }
}

register_fallible_client!(Client, bluetooth);
