use crate::channels::SyncSenderExt;
use crate::{arc_rw, read_lock, spawn, spawn_blocking, write_lock};
use color_eyre::{Report, Result};
use evdev_rs::enums::{int_to_ev_key, EventCode, EV_KEY, EV_LED};
use evdev_rs::DeviceWrapper;
use input::event::keyboard::{KeyState, KeyboardEventTrait};
use input::event::{DeviceEvent, EventTrait, KeyboardEvent};
use input::{DeviceCapability, Libinput, LibinputInterface};
use libc::{O_ACCMODE, O_RDONLY, O_RDWR};
use std::fs::{File, OpenOptions};
use std::os::unix::{fs::OpenOptionsExt, io::OwnedFd};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::broadcast;
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

pub struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        // No idea what these flags do honestly, just copied them from the example.
        let op = OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_ACCMODE == O_RDONLY) | (flags & O_ACCMODE == O_RDWR))
            .open(path)
            .map(OwnedFd::from);

        if let Err(err) = &op {
            error!("error opening {}: {err:?}", path.display());
        }

        op.map_err(|err| err.raw_os_error().unwrap_or(-1))
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(File::from(fd));
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
    pub fn init(seat: String) -> Arc<Self> {
        let client = Arc::new(Self::new(seat));
        {
            let client = client.clone();
            spawn_blocking(move || {
                if let Err(err) = client.run() {
                    error!("{err:?}");
                }
            });
        }
        client
    }

    fn new(seat: String) -> Self {
        let (tx, rx) = broadcast::channel(4);

        Self {
            tx,
            _rx: rx,
            seat,
            known_devices: arc_rw!(vec![]),
        }
    }

    fn run(&self) -> Result<()> {
        let mut input = Libinput::new_with_udev(Interface);
        input
            .udev_assign_seat(&self.seat)
            .map_err(|()| Report::msg("failed to assign seat"))?;

        loop {
            input.dispatch()?;

            for event in &mut input {
                match event {
                    input::Event::Keyboard(KeyboardEvent::Key(event))
                        if event.key_state() == KeyState::Released =>
                    {
                        let Some(device) = (unsafe { event.device().udev_device() }) else {
                            continue;
                        };

                        let Some(
                            key @ (EV_KEY::KEY_CAPSLOCK
                            | EV_KEY::KEY_NUMLOCK
                            | EV_KEY::KEY_SCROLLLOCK),
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
                                    tx.send_expect(event);
                                }
                            });
                        }
                    }
                    input::Event::Device(DeviceEvent::Added(event)) => {
                        let device = event.device();
                        if !device.has_capability(DeviceCapability::Keyboard) {
                            continue;
                        }

                        let name = device.name();
                        let Some(device) = (unsafe { event.device().udev_device() }) else {
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
                                debug!("new keyboard device: {name} | {}", device_path.display());
                                write_lock!(self.known_devices).push(device_path.to_path_buf());
                                self.tx.send_expect(Event::Device);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
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
