mod dbus_keyboard;
mod dbus_login1;
mod fs_reader;
#[cfg(feature = "ipc")]
mod ironvar;

use crate::{
    channels::SyncSenderExt,
    clients::brightness::fs_reader::{ReadParseError, scaling},
    lock, register_fallible_client, spawn,
};
use dbus_keyboard::KbdBacklightProxy;
use dbus_login1::Login1SessionProxy;
use futures_lite::StreamExt;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::broadcast,
    task::JoinHandle,
    time::{Duration, interval},
};
use tracing::{error, warn};

pub use fs_reader::{brightness, default_resource_name, max_brightness};

#[derive(Debug)]
pub(crate) enum BrightnessError {
    /// Zbus Error
    Zbus(zbus::Error),

    /// Reading file system error.
    Fs(ReadParseError),

    /// Could not find an already tracked device
    NoDeviceFound(Box<str>),
}

impl From<zbus::Error> for BrightnessError {
    fn from(value: zbus::Error) -> Self {
        Self::Zbus(value)
    }
}

impl From<ReadParseError> for BrightnessError {
    fn from(value: ReadParseError) -> Self {
        Self::Fs(value)
    }
}

impl std::fmt::Display for BrightnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrightnessError::Zbus(error) => error.fmt(f),
            BrightnessError::Fs(read_parse_error) => read_parse_error.fmt(f),
            BrightnessError::NoDeviceFound(key) => {
                write!(f, "The selected device with key {key} is not tracked")
            }
        }
    }
}

impl From<zbus::Error> for super::Error<BrightnessError> {
    fn from(value: zbus::Error) -> Self {
        super::Error::Service(value.into())
    }
}

impl From<ReadParseError> for super::Error<BrightnessError> {
    fn from(value: ReadParseError) -> Self {
        super::Error::Service(value.into())
    }
}

/// Events sent when the brightness changes.
///
/// You can compare an [Event] to a [Key] using [Event::as_key].
#[derive(Debug, Clone)]
pub(crate) enum Event {
    KbdBrightness { path: Box<str>, brightness: i32 },
    SysFsBrightness { device: Box<str>, brightness: i32 },
}

impl Event {
    /// Compare the event to a [Key].
    ///
    /// Use it to check if the event is for your device. If so `true` is returned, false otherwise.
    pub(crate) fn cmp_key(&self, key: &Key) -> bool {
        match (self, key) {
            (Event::KbdBrightness { path, .. }, Key::Kbd { path: kpath }) => kpath == path,
            (
                Event::SysFsBrightness { device, .. },
                Key::SysFs {
                    device: kdevice, ..
                },
            ) => kdevice == device,
            _ => false,
        }
    }

    /// Get the brightness from the [Event]
    pub(crate) fn brightness(self) -> i32 {
        match self {
            Event::KbdBrightness { brightness, .. } => brightness,
            Event::SysFsBrightness { brightness, .. } => brightness,
        }
    }
}

#[derive(Debug)]
pub(crate) struct BrightnessState {
    pub(crate) tracked: Vec<BrightnessDeviceState>,
}

/// Represent the scale the device is using for the brightness value.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Scale {
    Linear,
    Unknown,
}

impl From<Box<str>> for Scale {
    fn from(value: Box<str>) -> Self {
        match value.trim() {
            "linear" => Self::Linear,
            "unknown" => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

/// State of a device being tracked by [Client].
#[derive(Debug)]
pub(crate) enum BrightnessDeviceState {
    DBus {
        /// Path of the device usually `/org/freedesktop/UPower/KbdBacklight/kbd_backlight`
        key: Box<str>,

        max_brightness: i32,
    },
    SysFs {
        /// `device` name
        key: Box<str>,

        max_brightness: i32,

        scale: Scale,
    },
}

impl BrightnessDeviceState {
    pub(crate) fn key(&self) -> &str {
        match self {
            BrightnessDeviceState::DBus { key, .. } => key,
            BrightnessDeviceState::SysFs { key, .. } => key,
        }
    }
}

/// Represents properties specific to a type of device.
#[derive(Debug)]
enum BrightnessTypeProperties {
    Kbd {
        proxy: KbdBacklightProxy<'static>,

        max_brightness: i32,
    },
    SysFs {
        scale: Scale,

        max_brightness: i32,
    },
}

/// Represent a device managed by the [Client].
#[derive(Debug)]
pub(crate) struct BrightnessDevice {
    properties: BrightnessTypeProperties,

    /// Handle for task responsible to watch brightness changes.
    handle: JoinHandle<()>,
}

/// The [Key] is used to create a unique name for each device.
///
/// It assumes that the device's name or path is unique.
#[derive(Debug, PartialEq)]
pub(crate) enum Key {
    Kbd {
        path: Box<str>,
    },
    SysFs {
        subsystem: Box<str>,
        device: Box<str>,
    },
}

impl Key {
    pub(crate) fn get(&self) -> &str {
        match self {
            Key::Kbd { path } => path,
            Key::SysFs { device, .. } => device,
        }
    }
}

#[derive(Debug)]
pub(crate) enum Client {
    Stopped {
        tx: broadcast::Sender<Event>,
        _rx: broadcast::Receiver<Event>,
    },
    Started {
        tx: broadcast::Sender<Event>,

        _rx: broadcast::Receiver<Event>,

        /// D-Bus connection to avoid reopening one every time we connect a new device.
        dbus: zbus::connection::Connection,

        /// D-Bus connection to Login1 for setting brightness for `/sys/class` devices.
        screen: Login1SessionProxy<'static>,

        /// A HashMap of all current devices with their D-Bus connection if they have one.
        ///
        /// [BrightnessTypeProperties::Kbd] use their `path` as a key.
        ///
        /// [BrightnessTypeProperties::SysFs] user their combined `subsystem` and `device` as a key.
        devices: Arc<Mutex<HashMap<Box<str>, BrightnessDevice>>>,
    },
}

impl Client {
    pub(crate) fn new() -> Self {
        let (tx, rx) = broadcast::channel(8);
        Self::Stopped { tx, _rx: rx }
    }

    fn tx(&self) -> &broadcast::Sender<Event> {
        match self {
            Client::Stopped { tx, .. } => tx,
            Client::Started { tx, .. } => tx,
        }
    }

    /// Set the new brightness for a device.
    ///
    /// When using [Key::Kbd] the device must have already been tracked.
    ///
    /// Setting the task will invoke [spawn] to avoid delays created by [Login1SessionProxy::set_brightness].
    pub(crate) async fn set_brightness(
        &self,
        key: &Key,
        brightness: i32,
    ) -> super::Result<(), BrightnessError> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started {
                screen, devices, ..
            } => {
                match key {
                    Key::Kbd { path } => {
                        let devices = lock!(devices);
                        let Some(device) = devices.get(path) else {
                            return Err(BrightnessError::NoDeviceFound(path.clone()).into());
                        };

                        if let BrightnessTypeProperties::Kbd { proxy, .. } = &device.properties {
                            let proxy = proxy.clone();
                            spawn(async move {
                                proxy.set_brightness(brightness).await.expect("test");
                            });
                        }
                    }
                    Key::SysFs { subsystem, device } => {
                        let subsystem = subsystem.clone();
                        let device = device.clone();
                        let screen = screen.clone();
                        spawn(async move {
                            let Err(err) = screen
                                .set_brightness(&subsystem, &device, brightness as u32) // kernel does not allow negative brightness.
                                .await
                            else {
                                return;
                            };

                            error!("could not set brightness: {err}");
                        });
                    }
                }

                Ok(())
            }
        }
    }

    /// Return the maximum brightness for a device.
    ///
    /// The device must be tracked otherwise [BrightnessError::NoDeviceFound] is returned.
    #[allow(unused)]
    pub(crate) fn max_brightness(&self, key: &Key) -> super::Result<i32, BrightnessError> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started { devices, .. } => {
                let devices = lock!(devices);
                let key = key.get();
                let Some(device) = devices.get(key) else {
                    return Err(BrightnessError::NoDeviceFound(Box::from(key)).into());
                };

                Ok(match device.properties {
                    BrightnessTypeProperties::Kbd { max_brightness, .. } => max_brightness,
                    BrightnessTypeProperties::SysFs { max_brightness, .. } => max_brightness,
                })
            }
        }
    }

    /// Return the [Scale] used by the device's driver.
    ///
    /// If the scale is not known or cannot be determined then [Scale::Unknown] is returned.
    #[allow(unused)]
    pub(crate) fn scale(&self, key: &Key) -> super::Result<Scale, BrightnessError> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started { devices, .. } => {
                let devices = lock!(devices);
                let key = key.get();
                let Some(device) = devices.get(key) else {
                    return Err(BrightnessError::NoDeviceFound(Box::from(key)).into());
                };

                Ok(match &device.properties {
                    BrightnessTypeProperties::Kbd { .. } => Scale::Unknown,
                    BrightnessTypeProperties::SysFs { scale, .. } => *scale,
                })
            }
        }
    }

    /// Track a new keyboard device.
    ///
    /// Opens a D-Bus connection from `path` to the keyboard device. This device can then latter be addressed by [Key::Kbd] using the same `path`.
    ///
    /// Returns the current brightness.
    pub(crate) async fn track_new_kbd(&self, path: &str) -> super::Result<i32, zbus::Error> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started {
                tx, dbus, devices, ..
            } => {
                let owned_path = zbus::ObjectPath::try_from(path)?.into_owned();
                let proxy = KbdBacklightProxy::builder(dbus)
                    .path(owned_path)
                    .build()
                    .await?;
                let mut brightness_update = proxy.receive_brightness_changed_with_source().await?;
                let max_brightness = proxy.get_max_brightness().await?;
                let brightness = proxy.get_brightness().await?;
                let tx = tx.clone();

                let name: Box<str> = Box::from(path);
                let name_ = name.clone();
                let handle = spawn(async move {
                    loop {
                        let Some(event) = brightness_update.next().await else {
                            continue;
                        };

                        let Ok(args) = event.args() else {
                            continue;
                        };

                        tx.send_expect(Event::KbdBrightness {
                            path: name_.clone(),
                            brightness: args.value,
                        });
                    }
                });

                let mut devices = lock!(devices);
                devices.insert(
                    name,
                    BrightnessDevice {
                        properties: BrightnessTypeProperties::Kbd {
                            proxy,
                            max_brightness,
                        },
                        handle,
                    },
                );

                Ok(brightness)
            }
        }
    }

    /// Track a new device in `/sys/class`.
    ///
    /// `subsystem` and `device` must match an already existing entry.
    /// The device can then later be addressed by using [Key::SysFs].
    ///
    /// Returns the current brightness.
    pub(crate) fn track_new_kernel_device(
        &self,
        subsystem: &str,
        device: &str,
    ) -> super::Result<i32, ReadParseError> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started {
                tx, _rx, devices, ..
            } => {
                let max_brightness = max_brightness(subsystem, device)?;
                let returned_brightness = brightness(subsystem, device)?;
                let scale: Box<str> = scaling(subsystem, device)?;

                tracing::info!("scale: {scale}");

                let subsystem: Box<str> = Box::from(subsystem);
                let device: Box<str> = Box::from(device);

                let key = Key::SysFs {
                    subsystem: subsystem.clone(),
                    device: device.clone(),
                };

                let tx = tx.clone();
                let mut prev_brightness = returned_brightness;
                let handle = spawn(async move {
                    const POLL_INTERVAL: Duration = Duration::from_millis(50);
                    let mut interval = interval(POLL_INTERVAL);

                    loop {
                        interval.tick().await;

                        let new_brightness = match brightness(&subsystem, &device) {
                            Ok(brightness) => brightness,
                            Err(err) => {
                                warn!("could not read brightness: {err}");
                                continue;
                            }
                        };

                        if prev_brightness != new_brightness {
                            tx.send_expect(Event::SysFsBrightness {
                                device: device.clone(),
                                brightness: new_brightness,
                            });
                            prev_brightness = new_brightness;
                        }
                    }
                });

                let mut devices = lock!(devices);
                devices.insert(
                    key.get().into(),
                    BrightnessDevice {
                        properties: BrightnessTypeProperties::SysFs {
                            scale: scale.into(),
                            max_brightness,
                        },
                        handle,
                    },
                );

                Ok(returned_brightness)
            }
        }
    }
}

impl super::Client for Client {
    type State = BrightnessState;

    type Event = Event;

    type Error = zbus::Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> Result<Self, Self::Error> {
        let Self::Stopped { tx, _rx } = self else {
            return Ok(self);
        };

        let dbus = Box::pin(zbus::Connection::system()).await?;
        let screen = Login1SessionProxy::new(&dbus).await?;

        let devices = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self::Started {
            tx,
            _rx,
            dbus,
            screen,
            devices,
        })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        let Self::Started {
            tx, _rx, devices, ..
        } = self
        else {
            return Ok(self);
        };

        let devices = lock!(devices);
        for device in devices.values() {
            device.handle.abort();
        }

        Ok(Self::Stopped { tx, _rx })
    }

    async fn state(&self) -> super::Result<Self::State, Self::Error> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started { devices, .. } => {
                let devices = lock!(devices);

                let mut device_states = Vec::with_capacity(devices.len());
                for (key, device) in devices.iter() {
                    let state = match &device.properties {
                        BrightnessTypeProperties::Kbd { max_brightness, .. } => {
                            BrightnessDeviceState::DBus {
                                key: key.clone(),
                                max_brightness: *max_brightness,
                            }
                        }
                        BrightnessTypeProperties::SysFs {
                            scale,
                            max_brightness,
                        } => BrightnessDeviceState::SysFs {
                            key: key.clone(),
                            max_brightness: *max_brightness,
                            scale: *scale,
                        },
                    };
                    device_states.push(state);
                }

                Ok(BrightnessState {
                    tracked: device_states,
                })
            }
        }
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

register_fallible_client!(Client, brightness);
