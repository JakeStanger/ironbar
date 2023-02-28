use super::device::{DataControlDevice, DataControlDeviceEvent};
use super::source::DataControlSource;
use smithay_client_toolkit::data_device::WritePipe;
use smithay_client_toolkit::environment::{Environment, GlobalHandler};
use smithay_client_toolkit::seat::{SeatHandling, SeatListener};
use smithay_client_toolkit::MissingGlobal;
use std::cell::RefCell;
use std::rc::{self, Rc};
use tracing::warn;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Attached, DispatchData};
use wayland_protocols::wlr::unstable::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;

enum DataControlDeviceHandlerInner {
    Ready {
        manager: Attached<ZwlrDataControlManagerV1>,
        devices: Vec<(WlSeat, DataControlDevice)>,
        status_listeners: Rc<RefCell<Vec<rc::Weak<RefCell<DataControlDeviceStatusCallback>>>>>,
    },
    Pending {
        seats: Vec<WlSeat>,
        status_listeners: Rc<RefCell<Vec<rc::Weak<RefCell<DataControlDeviceStatusCallback>>>>>,
    },
}

impl DataControlDeviceHandlerInner {
    fn init_manager(&mut self, manager: Attached<ZwlrDataControlManagerV1>) {
        let (seats, status_listeners) = if let Self::Pending {
            seats,
            status_listeners,
        } = self
        {
            (std::mem::take(seats), status_listeners.clone())
        } else {
            warn!("Ignoring second zwlr_data_control_manager_v1");
            return;
        };

        let mut devices = Vec::new();

        for seat in seats {
            let my_seat = seat.clone();
            let status_listeners = status_listeners.clone();
            let device =
                DataControlDevice::init_for_seat(&manager, &seat, move |event, dispatch_data| {
                    notify_status_listeners(&my_seat, &event, dispatch_data, &status_listeners);
                });
            devices.push((seat.clone(), device));
        }

        *self = Self::Ready {
            manager,
            devices,
            status_listeners,
        };
    }

    fn get_manager(&self) -> Option<Attached<ZwlrDataControlManagerV1>> {
        match self {
            Self::Ready { manager, .. } => Some(manager.clone()),
            Self::Pending { .. } => None,
        }
    }

    fn new_seat(&mut self, seat: &WlSeat) {
        match self {
            Self::Ready {
                manager,
                devices,
                status_listeners,
            } => {
                if devices.iter().any(|(s, _)| s == seat) {
                    // the seat already exists, nothing to do
                    return;
                }
                let my_seat = seat.clone();
                let status_listeners = status_listeners.clone();
                let device =
                    DataControlDevice::init_for_seat(manager, seat, move |event, dispatch_data| {
                        notify_status_listeners(&my_seat, &event, dispatch_data, &status_listeners);
                    });
                devices.push((seat.clone(), device));
            }
            Self::Pending { seats, .. } => {
                seats.push(seat.clone());
            }
        }
    }

    fn remove_seat(&mut self, seat: &WlSeat) {
        match self {
            Self::Ready { devices, .. } => devices.retain(|(s, _)| s != seat),
            Self::Pending { seats, .. } => seats.retain(|s| s != seat),
        }
    }

    fn create_source<F>(&self, mime_types: Vec<String>, callback: F) -> Option<DataControlSource>
    where
        F: FnMut(String, WritePipe, DispatchData) + 'static,
    {
        match self {
            Self::Ready { manager, .. } => {
                let source = DataControlSource::new(manager, mime_types, callback);
                Some(source)
            }
            Self::Pending { .. } => None,
        }
    }

    fn with_device<F>(&self, seat: &WlSeat, f: F) -> Result<(), MissingGlobal>
    where
        F: FnOnce(&DataControlDevice),
    {
        match self {
            Self::Ready { devices, .. } => {
                let device = devices
                    .iter()
                    .find_map(|(s, device)| if s == seat { Some(device) } else { None });

                device.map_or(Err(MissingGlobal), |device| {
                    f(device);
                    Ok(())
                })
            }
            Self::Pending { .. } => Err(MissingGlobal),
        }
    }
}

pub struct DataControlDeviceHandler {
    inner: Rc<RefCell<DataControlDeviceHandlerInner>>,
    status_listeners: Rc<RefCell<Vec<rc::Weak<RefCell<DataControlDeviceStatusCallback>>>>>,
    _seat_listener: SeatListener,
}

impl DataControlDeviceHandler {
    pub fn init<S>(seat_handler: &mut S) -> Self
    where
        S: SeatHandling,
    {
        let status_listeners = Rc::new(RefCell::new(Vec::new()));

        let inner = Rc::new(RefCell::new(DataControlDeviceHandlerInner::Pending {
            seats: Vec::new(),
            status_listeners: status_listeners.clone(),
        }));

        let seat_inner = inner.clone();
        let seat_listener = seat_handler.listen(move |seat, seat_data, _| {
            if seat_data.defunct {
                seat_inner.borrow_mut().remove_seat(&seat);
            } else {
                seat_inner.borrow_mut().new_seat(&seat);
            }
        });

        Self {
            inner,
            _seat_listener: seat_listener,
            status_listeners,
        }
    }
}

impl GlobalHandler<ZwlrDataControlManagerV1> for DataControlDeviceHandler {
    fn created(
        &mut self,
        registry: Attached<WlRegistry>,
        id: u32,
        version: u32,
        _ddata: DispatchData,
    ) {
        // data control manager is supported until version 2
        let version = std::cmp::min(version, 2);

        let manager = registry.bind::<ZwlrDataControlManagerV1>(version, id);
        self.inner.borrow_mut().init_manager((*manager).clone());
    }

    fn get(&self) -> Option<Attached<ZwlrDataControlManagerV1>> {
        RefCell::borrow(&self.inner).get_manager()
    }
}

type DataControlDeviceStatusCallback =
    dyn FnMut(WlSeat, DataControlDeviceEvent, DispatchData) + 'static;

/// Notifies the callbacks of an event on the data device
fn notify_status_listeners(
    seat: &WlSeat,
    event: &DataControlDeviceEvent,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<DataControlDeviceStatusCallback>>>>,
) {
    listeners.borrow_mut().retain(|lst| {
        rc::Weak::upgrade(lst).map_or(false, |cb| {
            (cb.borrow_mut())(seat.clone(), event.clone(), ddata.reborrow());
            true
        })
    });
}

pub struct DataControlDeviceStatusListener {
    _cb: Rc<RefCell<DataControlDeviceStatusCallback>>,
}

pub trait DataControlDeviceHandling {
    fn listen<F>(&mut self, f: F) -> DataControlDeviceStatusListener
    where
        F: FnMut(WlSeat, DataControlDeviceEvent, DispatchData) + 'static;

    fn with_data_control_device<F>(&self, seat: &WlSeat, f: F) -> Result<(), MissingGlobal>
    where
        F: FnOnce(&DataControlDevice);

    fn create_source<F>(&self, mime_types: Vec<String>, callback: F) -> Option<DataControlSource>
    where
        F: FnMut(String, WritePipe, DispatchData) + 'static;
}

impl DataControlDeviceHandling for DataControlDeviceHandler {
    fn listen<F>(&mut self, f: F) -> DataControlDeviceStatusListener
    where
        F: FnMut(WlSeat, DataControlDeviceEvent, DispatchData) + 'static,
    {
        let rc = Rc::new(RefCell::new(f)) as Rc<_>;
        self.status_listeners.borrow_mut().push(Rc::downgrade(&rc));
        DataControlDeviceStatusListener { _cb: rc }
    }

    fn with_data_control_device<F>(&self, seat: &WlSeat, f: F) -> Result<(), MissingGlobal>
    where
        F: FnOnce(&DataControlDevice),
    {
        RefCell::borrow(&self.inner).with_device(seat, f)
    }

    fn create_source<F>(&self, mime_types: Vec<String>, callback: F) -> Option<DataControlSource>
    where
        F: FnMut(String, WritePipe, DispatchData) + 'static,
    {
        RefCell::borrow(&self.inner).create_source(mime_types, callback)
    }
}

pub fn listen_to_devices<E, F>(env: &Environment<E>, f: F) -> DataControlDeviceStatusListener
where
    E: DataControlDeviceHandling,
    F: FnMut(WlSeat, DataControlDeviceEvent, DispatchData) + 'static,
{
    env.with_inner(move |inner| DataControlDeviceHandling::listen(inner, f))
}
