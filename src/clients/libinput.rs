use crate::channels::SyncSenderExt;
use crate::{Ironbar, arc_rw, read_lock, spawn, write_lock};
use colpetto::event::{AsRawEvent, DeviceEvent, KeyState, KeyboardEvent};
use colpetto::{DeviceCapability, EventStream, Libinput};
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
use thiserror::Error;
use tokio::sync::{broadcast, oneshot};
use tokio::task::LocalSet;
use tokio::time::sleep;
use tracing::{debug, error, warn};

#[derive(Debug, Error)]
pub enum Error {
    #[error("provided key is not a supported toggle key: {0}")]
    UnsupportedKey(EV_KEY),
    #[error("unable to fetch key state")]
    KeyStateFetch,
    #[error("unexpected end of stream")]
    EndOfStream,
    #[error("libinput thread exited before initializing")]
    ThreadExited,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Ffi(#[from] std::ffi::NulError),
    #[error(transparent)]
    LibInput(#[from] colpetto::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

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
    type Error = Error;

    fn try_from(value: EV_KEY) -> std::result::Result<Self, Self::Error> {
        match value {
            EV_KEY::KEY_CAPSLOCK => Ok(Key::Caps),
            EV_KEY::KEY_NUMLOCK => Ok(Key::Num),
            EV_KEY::KEY_SCROLLLOCK => Ok(Key::Scroll),
            _ => Err(Error::UnsupportedKey(value)),
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
        .ok_or(Error::KeyStateFetch)
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

#[allow(dead_code)]
pub struct State {
    pub caps: bool,
    pub num: bool,
    pub scroll: bool,
}

struct KeyData<P: AsRef<Path>> {
    device_path: P,
    key: EV_KEY,
}

impl<P: AsRef<Path>> TryFrom<KeyData<P>> for Event {
    type Error = Error;

    fn try_from(data: KeyData<P>) -> Result<Self> {
        let key = Key::try_from(data.key)?;

        key.get_state(data.device_path)
            .map(|state| KeyEvent { key, state })
            .map(Event::Key)
    }
}

#[derive(Debug)]
pub enum Client {
    Stopped {
        seat: String,
        tx: broadcast::Sender<Event>,
        _rx: broadcast::Receiver<Event>,
    },
    Started {
        seat: String,
        tx: broadcast::Sender<Event>,
        _rx: broadcast::Receiver<Event>,
        known_devices: Arc<RwLock<Vec<PathBuf>>>,
        stop_tx: oneshot::Sender<()>,
    },
}

impl Client {
    pub(super) fn new(seat: &str) -> Self {
        let (tx, rx) = broadcast::channel(4);
        Self::Stopped {
            seat: seat.to_owned(),
            tx,
            _rx: rx,
        }
    }

    fn open_restricted(path: &CStr, flags: c_int) -> std::result::Result<RawFd, i32> {
        open(path, OFlags::from_bits_retain(flags as u32), Mode::empty())
            .map(IntoRawFd::into_raw_fd)
            .map_err(Errno::raw_os_error)
    }

    fn close_restricted(fd: c_int) {
        drop(unsafe { OwnedFd::from_raw_fd(fd) });
    }

    /// Creates the libinput context for `seat`
    /// and opens an event stream on it.
    ///
    /// The returned stream is `!Send`,
    /// so this must be called from the libinput thread.
    fn connect(seat: &str) -> Result<EventStream> {
        let mut libinput = Libinput::with_tracing(Self::open_restricted, Self::close_restricted)?;
        libinput.udev_assign_seat(CString::new(seat)?.as_c_str())?;
        Ok(libinput.event_stream()?)
    }

    async fn run(
        mut stream: EventStream,
        tx: &broadcast::Sender<Event>,
        known_devices: &RwLock<Vec<PathBuf>>,
    ) -> Result<()> {
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
                            write_lock!(known_devices).push(device_path.to_path_buf());
                            tx.send_expect(Event::Device);
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
                        let tx = tx.clone();

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
                _ => {}
            }
        }

        Err(Error::EndOfStream)
    }

    fn tx(&self) -> &broadcast::Sender<Event> {
        match self {
            Self::Stopped { tx, .. } => tx,
            Self::Started { tx, .. } => tx,
        }
    }

    pub fn get_state(&self, key: Key) -> bool {
        let Self::Started { known_devices, .. } = self else {
            return false;
        };

        read_lock!(known_devices)
            .iter()
            .map(|device_path| key.get_state(device_path))
            .filter_map(Result::ok)
            .reduce(|state, curr| state || curr)
            .unwrap_or_default()
    }
}

impl super::Client for Client {
    type State = State;
    type Event = Event;
    type Error = Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> std::result::Result<Self, Self::Error> {
        let Self::Stopped { seat, tx, _rx } = self else {
            return Ok(self);
        };

        let known_devices = arc_rw!(vec![]);

        let (ready_tx, ready_rx) = oneshot::channel();
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        // libinput types are `!Send`,
        // so the event loop lives on a dedicated thread with its own `LocalSet`.
        {
            let seat = seat.clone();
            let tx = tx.clone();
            let known_devices = known_devices.clone();

            std::thread::spawn(move || {
                let local = LocalSet::new();

                local.spawn_local(async move {
                    let stream = match Self::connect(&seat) {
                        Ok(stream) => {
                            ready_tx.send(Ok(())).expect(crate::error::ERR_CHANNEL_SEND);
                            stream
                        }
                        Err(err) => {
                            ready_tx
                                .send(Err(err))
                                .expect(crate::error::ERR_CHANNEL_SEND);
                            return;
                        }
                    };

                    tokio::select! {
                        res = Self::run(stream, &tx, &known_devices) => {
                            if let Err(err) = res {
                                error!("{err:?}");
                            }
                        }
                        // resolves on explicit stop, or when the client is dropped
                        _ = stop_rx => debug!("stopping libinput client for seat '{seat}'"),
                    }
                });

                Ironbar::runtime().block_on(local);
            });
        }

        ready_rx.await.map_err(|_| Error::ThreadExited)??;

        Ok(Self::Started {
            seat,
            tx,
            _rx,
            known_devices,
            stop_tx,
        })
    }

    async fn stop(self) -> std::result::Result<Self, Self::Error> {
        let Self::Started {
            seat,
            tx,
            _rx,
            stop_tx,
            ..
        } = self
        else {
            return Ok(self);
        };

        if stop_tx.send(()).is_err() {
            warn!(
                "failed to send stop signal to libinput thread - stream may have already exited unexpectedly"
            );
        }

        Ok(Self::Stopped { seat, tx, _rx })
    }

    async fn state(&self) -> super::Result<Self::State, Self::Error> {
        if !self.is_started() {
            return Err(super::Error::InvalidState);
        }

        let caps = self.get_state(Key::Caps);
        let num = self.get_state(Key::Num);
        let scroll = self.get_state(Key::Scroll);

        Ok(Self::State { caps, num, scroll })
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}
