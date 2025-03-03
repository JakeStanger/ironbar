use crate::clients::ClientResult;
use crate::{arc_rw, read_lock, send, spawn, write_lock};
use color_eyre::{Report, Result};
use colpetto::event::{AsRawEvent, DeviceEvent, KeyState, KeyboardEvent};
use colpetto::{DeviceCapability, Libinput};
use evdev_rs::DeviceWrapper;
use evdev_rs::enums::{EV_KEY, EV_LED, EventCode, int_to_ev_key};
use futures_lite::StreamExt;
use rustix::fs::{Mode, OFlags, open};
use rustix::io::Errno;
use std::ffi::{CStr, CString, c_int};
use std::os::fd::{FromRawFd, IntoRawFd, RawFd};
use std::os::unix::io::OwnedFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::LocalSet;
use tokio::time::sleep;
use tracing::{debug, error};

#[derive(Debug, Copy, Clone)]
pub enum Key {
    Caps,
    Num,
    Scroll,
}

impl From<Key> for EV_KEY {
    fn from(value: Key) -> Self {
        match value {
            Key::Caps => Self::KEY_CAPSLOCK,
            Key::Num => Self::KEY_NUMLOCK,
            Key::Scroll => Self::KEY_SCROLLLOCK,
        }
    }
}

impl TryFrom<EV_KEY> for Key {
    type Error = Report;

    fn try_from(value: EV_KEY) -> std::result::Result<Self, Self::Error> {
        match value {
            EV_KEY::KEY_CAPSLOCK => Ok(Key::Caps),
            EV_KEY::KEY_NUMLOCK => Ok(Key::Num),
            EV_KEY::KEY_SCROLLLOCK => Ok(Key::Scroll),
            _ => Err(Report::msg("provided key is not supported toggle key")),
        }
    }
}

impl Key {
    fn get_state<P: AsRef<Path>>(self, device_path: P) -> Result<bool> {
        let device = evdev_rs::Device::new_from_path(device_path)?;

        match self {
            Self::Caps => device.event_value(&EventCode::EV_LED(EV_LED::LED_CAPSL)),
            Self::Num => device.event_value(&EventCode::EV_LED(EV_LED::LED_NUML)),
            Self::Scroll => device.event_value(&EventCode::EV_LED(EV_LED::LED_SCROLLL)),
        }
        .map(|v| v > 0)
        .ok_or_else(|| Report::msg("failed to get key status"))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct KeyEvent {
    pub key: Key,
    pub state: bool,
}

#[derive(Debug, Copy, Clone)]
pub enum Event {
    Device,
    Key(KeyEvent),
}

struct KeyData<P: AsRef<Path>> {
    device_path: P,
    key: EV_KEY,
}

impl<P: AsRef<Path>> TryFrom<KeyData<P>> for Event {
    type Error = Report;

    fn try_from(data: KeyData<P>) -> Result<Self> {
        let key = Key::try_from(data.key)?;

        key.get_state(data.device_path)
            .map(|state| KeyEvent { key, state })
            .map(Event::Key)
    }
}

#[derive(Debug)]
pub struct Client {
    tx: broadcast::Sender<Event>,
    _rx: broadcast::Receiver<Event>,

    seat: String,
    known_devices: Arc<RwLock<Vec<PathBuf>>>,
}

impl Client {
    pub fn init(seat: String) -> ClientResult<Self> {
        let client = Arc::new(Self::new(seat)?);

        {
            let client = client.clone();
            let local = LocalSet::new();

            local.spawn_local(async move {
                if let Err(err) = client.run().await {
                    error!("{err:?}");
                }
            });
        }

        Ok(client)
    }

    fn new(seat: String) -> Result<Self> {
        let (tx, rx) = broadcast::channel(4);

        Ok(Self {
            tx,
            _rx: rx,
            seat,
            known_devices: arc_rw!(vec![]),
        })
    }

    fn open_restricted(path: &CStr, flags: c_int) -> std::result::Result<RawFd, i32> {
        open(path, OFlags::from_bits_retain(flags as u32), Mode::empty())
            .map(IntoRawFd::into_raw_fd)
            .map_err(Errno::raw_os_error)
    }

    fn close_restricted(fd: c_int) {
        drop(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    async fn run(&self) -> Result<()> {
        let mut libinput = Libinput::with_logger(
            Self::open_restricted,
            Self::close_restricted,
            Some(colpetto::tracing_logger),
        )?;

        libinput.udev_assign_seat(CString::new(&*self.seat)?.as_c_str())?;

        let mut stream = libinput.event_stream()?;
        while let Some(event) = stream.try_next().await? {
            match event {
                colpetto::Event::Device(DeviceEvent::Added(event)) => {
                    let device = event.device();
                    if !device.has_capability(DeviceCapability::Keyboard) {
                        continue;
                    }

                    let name = device.name();
                    let Some(device) = event.device().udev_device() else {
                        continue;
                    };

                    if let Some(device_path) = device.devnode() {
                        // not all devices which report as keyboards actually are one -
                        // fire test event so we can figure out if it is
                        let caps_event: Result<Event> = KeyData {
                            device_path,
                            key: EV_KEY::KEY_CAPSLOCK,
                        }
                        .try_into();

                        if caps_event.is_ok() {
                            debug!(
                                "new keyboard device: {} | {}",
                                name.to_string_lossy(),
                                device_path.display()
                            );
                            write_lock!(self.known_devices).push(device_path.to_path_buf());
                            send!(self.tx, Event::Device);
                        }
                    }
                }
                colpetto::Event::Keyboard(KeyboardEvent::Key(event))
                    if event.key_state() == KeyState::Released =>
                {
                    let Some(device) = event.device().udev_device() else {
                        continue;
                    };

                    let Some(
                        key @ (EV_KEY::KEY_CAPSLOCK | EV_KEY::KEY_NUMLOCK | EV_KEY::KEY_SCROLLLOCK),
                    ) = int_to_ev_key(event.key())
                    else {
                        continue;
                    };

                    if let Some(device_path) = device.devnode().map(PathBuf::from) {
                        let tx = self.tx.clone();

                        // need to spawn a task to avoid blocking
                        spawn(async move {
                            // wait for kb to change
                            sleep(Duration::from_millis(50)).await;

                            let data = KeyData { device_path, key };

                            if let Ok(event) = data.try_into() {
                                send!(tx, event);
                            }
                        });
                    }
                }
                _ => {}
            }
        }

        Err(Report::msg("unexpected end of stream"))
    }

    pub fn get_state(&self, key: Key) -> bool {
        read_lock!(self.known_devices)
            .iter()
            .map(|device_path| key.get_state(device_path))
            .filter_map(Result::ok)
            .reduce(|state, curr| state || curr)
            .unwrap_or_default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}
