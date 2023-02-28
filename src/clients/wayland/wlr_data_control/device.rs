use super::offer::DataControlOffer;
use super::source::DataControlSource;
use crate::lock;
use std::sync::{Arc, Mutex};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Attached, DispatchData, Main};
use wayland_protocols::wlr::unstable::data_control::v1::client::{
    zwlr_data_control_device_v1::{Event, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
};

#[derive(Debug)]
struct Inner {
    offer: Option<Arc<DataControlOffer>>,
}

impl Inner {
    fn new_offer(&mut self, offer: &Main<ZwlrDataControlOfferV1>) {
        self.offer.replace(Arc::new(DataControlOffer::new(offer)));
    }
}

#[derive(Debug, Clone)]
pub struct DataControlDeviceEvent(pub Arc<DataControlOffer>);

fn data_control_device_implem<F>(
    event: Event,
    inner: &mut Inner,
    implem: &mut F,
    ddata: DispatchData,
) where
    F: FnMut(DataControlDeviceEvent, DispatchData),
{
    match event {
        Event::DataOffer { id } => {
            inner.new_offer(&id);
        }
        Event::Selection { id: Some(offer) } => {
            let inner_offer = inner
                .offer
                .clone()
                .expect("Offer should exist at this stage");
            if offer == inner_offer.offer {
                implem(DataControlDeviceEvent(inner_offer), ddata);
            }
        }
        _ => {}
    }
}

pub struct DataControlDevice {
    device: ZwlrDataControlDeviceV1,
    _inner: Arc<Mutex<Inner>>,
}

impl DataControlDevice {
    pub fn init_for_seat<F>(
        manager: &Attached<ZwlrDataControlManagerV1>,
        seat: &WlSeat,
        mut callback: F,
    ) -> Self
    where
        F: FnMut(DataControlDeviceEvent, DispatchData) + 'static,
    {
        let inner = Arc::new(Mutex::new(Inner { offer: None }));

        let device = manager.get_data_device(seat);

        {
            let inner = inner.clone();
            device.quick_assign(move |_handle, event, ddata| {
                let mut inner = lock!(inner);
                data_control_device_implem(event, &mut inner, &mut callback, ddata);
            });
        }

        Self {
            device: device.detach(),
            _inner: inner,
        }
    }

    pub fn set_selection(&self, source: &Option<DataControlSource>) {
        self.device
            .set_selection(source.as_ref().map(|s| &s.source));
    }
}
