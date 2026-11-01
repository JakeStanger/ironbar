use crate::channels::{BroadcastChannel, SyncSenderExt};
use crate::{Ironbar, arc_mut, error, lock, register_fallible_client, spawn, stop_basic};
use bluer::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::{JoinHandle, JoinSet};
use tracing::debug;

type Handles = (
    bluer::Session,
    Arc<Mutex<BluetoothState>>,
    broadcast::Sender<BluetoothState>,
);

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

#[derive(Debug)]
pub enum Client {
    Stopped(BroadcastChannel<BluetoothState>),
    Started {
        channel: BroadcastChannel<BluetoothState>,
        session: bluer::Session,
        state: Arc<Mutex<BluetoothState>>,
        handle: JoinHandle<()>,
    },
}

impl Client {
    pub(in crate::clients) fn new() -> Self {
        Self::Stopped(BroadcastChannel::new(16))
    }

    fn tx(&self) -> &broadcast::Sender<BluetoothState> {
        match self {
            Self::Stopped(channel) => channel,
            Self::Started { channel, .. } => channel,
        }
    }

    /// Clones out the session, state and sender
    /// required by the action methods.
    fn handles(&self) -> super::Result<Handles, bluer::Error> {
        match self {
            Self::Stopped(_) => Err(super::Error::InvalidState),
            Self::Started {
                channel,
                session,
                state,
                ..
            } => Ok((session.clone(), state.clone(), (*channel).clone())),
        }
    }

    /// Runs `f` against the current state,
    /// broadcasting the new state if `f` returns `true`.
    ///
    /// Returns the result of `f`.
    fn update_state(
        state: &Mutex<BluetoothState>,
        tx: &broadcast::Sender<BluetoothState>,
        f: impl FnOnce(&mut BluetoothState) -> bool,
    ) -> bool {
        let mut state = lock!(state);
        let modified = f(&mut state);
        if modified {
            tx.send_expect(state.clone());
        }
        modified
    }

    pub(crate) fn enable(&self) -> super::Result<(), bluer::Error> {
        debug!("Enabling");

        let (session, state, tx) = self.handles()?;
        spawn(async move {
            if !Self::update_state(&state, &tx, |state| {
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

            Self::update_state(&state, &tx, |state| {
                if matches!(state, BluetoothState::Enabling) {
                    *state = new_state;
                    true
                } else {
                    false
                }
            });
        });

        Ok(())
    }

    pub(crate) fn disable(&self) -> super::Result<(), bluer::Error> {
        debug!("Disabling");

        let (session, state, tx) = self.handles()?;
        spawn(async move {
            if !Self::update_state(&state, &tx, |state| {
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

            Self::update_state(&state, &tx, |state| {
                if matches!(state, BluetoothState::Disabling) {
                    *state = new_state;
                    true
                } else {
                    false
                }
            });
        });

        Ok(())
    }

    pub(crate) fn connect(&self, address: bluer::Address) -> super::Result<(), bluer::Error> {
        debug!("Connect {}", address);

        let (session, state, tx) = self.handles()?;
        spawn(async move {
            if !Self::update_state(&state, &tx, |state| {
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

            Self::update_state(&state, &tx, |state| {
                Self::try_replace_device_status(
                    state,
                    address,
                    BluetoothDeviceStatus::Connecting,
                    new_status,
                )
            });
        });

        Ok(())
    }

    pub(crate) fn disconnect(&self, address: bluer::Address) -> super::Result<(), bluer::Error> {
        debug!("Disconnect {}", address);

        let (session, state, tx) = self.handles()?;
        spawn(async move {
            if !Self::update_state(&state, &tx, |state| {
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

            Self::update_state(&state, &tx, |state| {
                Self::try_replace_device_status(
                    state,
                    address,
                    BluetoothDeviceStatus::Disconnecting,
                    new_status,
                )
            });
        });

        Ok(())
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
            // bluez is not running, or cannot be activated
            Err(bluer::Error {
                kind: bluer::ErrorKind::Internal(bluer::InternalErrorKind::DBus(name)),
                ..
            }) if matches!(
                name.as_str(),
                "org.freedesktop.DBus.Error.NameHasNoOwner"
                    | "org.freedesktop.DBus.Error.ServiceUnknown"
            ) =>
            {
                BluetoothState::NotFound
            }
            Err(err) => return Err(err),
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

impl super::Client for Client {
    type State = BluetoothState;
    type Event = BluetoothState;
    type Error = bluer::Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> std::result::Result<Self, Self::Error> {
        let Self::Stopped(channel) = self else {
            return Ok(self);
        };

        debug!("Starting bluetooth session");

        let session = bluer::Session::new().await?;
        let initial_state = Self::get_state(&session).await.unwrap_or_else(|err| {
            error!("Bluetooth error: {}", err);
            BluetoothState::NotFound
        });
        let state = arc_mut!(initial_state);

        let handle = spawn({
            let session = session.clone();
            let state = state.clone();
            let tx = channel.clone();

            async move {
                // Ideally, this loop should be replaced by a subscription to
                // Bluetooth adapter events (see `bluer::Adapter::events`).
                // But unfortunately it does not work very reliably,
                // sometimes missing important events.
                // Instead, we just get a full state very 3 seconds.
                //
                // This does not affect responsiveness, as user actions force
                // an immediate state update.
                loop {
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    match Self::get_state(&session).await {
                        Ok(mut new_state) => {
                            debug!("New bluetooth state: {:?}", new_state);

                            Self::update_state(&state, &tx, |old_state| {
                                Self::patch_new_state(&mut new_state, old_state);
                                if *old_state == new_state {
                                    false
                                } else {
                                    *old_state = new_state;
                                    true
                                }
                            });
                        }
                        Err(err) => {
                            error!("Bluetooth error: {}", err);
                        }
                    }
                }
            }
        });

        Ok(Self::Started {
            channel,
            session,
            state,
            handle,
        })
    }

    async fn stop(self) -> std::result::Result<Self, Self::Error> {
        debug!("Stopping bluetooth session");
        stop_basic!(self)
    }

    async fn state(&self) -> super::Result<Self::State, Self::Error> {
        match self {
            Self::Stopped(_) => Err(super::Error::InvalidState),
            Self::Started { state, .. } => Ok(lock!(state).clone()),
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

register_fallible_client!(Client, bluetooth);
