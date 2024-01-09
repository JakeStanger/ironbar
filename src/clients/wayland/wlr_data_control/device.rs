use super::manager::DataControlDeviceManagerState;
use super::offer::{
    DataControlOfferData, DataControlOfferDataExt, DataControlOfferHandler, SelectionOffer,
};
use crate::error::ERR_WAYLAND_DATA;
use crate::lock;
use std::sync::{Arc, Mutex};
use tracing::warn;
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{Event, ZwlrDataControlDeviceV1},
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
};

#[derive(Debug)]
pub struct DataControlDevice {
    pub device: ZwlrDataControlDeviceV1,
}

#[derive(Debug, Default)]
pub struct DataControlDeviceInner {
    /// the active selection offer and its data
    selection_offer: Arc<Mutex<Option<ZwlrDataControlOfferV1>>>,
    /// the active undetermined offers and their data
    pub undetermined_offers: Arc<Mutex<Vec<ZwlrDataControlOfferV1>>>,
}

#[derive(Debug, Default)]
pub struct DataControlDeviceData {
    pub(super) inner: Arc<Mutex<DataControlDeviceInner>>,
}

pub trait DataControlDeviceDataExt: Send + Sync {
    type DataControlOfferInner: DataControlOfferDataExt + Send + Sync + 'static;

    fn data_control_device_data(&self) -> &DataControlDeviceData;

    fn selection_mime_types(&self) -> Vec<String> {
        let inner = self.data_control_device_data();
        lock!(lock!(inner.inner).selection_offer)
            .as_ref()
            .map(|offer| {
                let data = offer
                    .data::<Self::DataControlOfferInner>()
                    .expect(ERR_WAYLAND_DATA);
                data.mime_types()
            })
            .unwrap_or_default()
    }

    /// Get the active selection offer if it exists.
    fn selection_offer(&self) -> Option<SelectionOffer> {
        let inner = self.data_control_device_data();
        lock!(lock!(inner.inner).selection_offer)
            .as_ref()
            .and_then(|offer| {
                let data = offer
                    .data::<Self::DataControlOfferInner>()
                    .expect(ERR_WAYLAND_DATA);
                data.as_selection_offer()
            })
    }
}

impl DataControlDeviceDataExt for DataControlDevice {
    type DataControlOfferInner = DataControlOfferData;
    fn data_control_device_data(&self) -> &DataControlDeviceData {
        self.device.data().expect(ERR_WAYLAND_DATA)
    }
}

impl DataControlDeviceDataExt for DataControlDeviceData {
    type DataControlOfferInner = DataControlOfferData;
    fn data_control_device_data(&self) -> &DataControlDeviceData {
        self
    }
}

/// Handler trait for `DataDevice` events.
///
/// The functions defined in this trait are called as `DataDevice` events are received from the compositor.
pub trait DataControlDeviceHandler: Sized {
    /// Advertises a new selection.
    fn selection(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        data_device: DataControlDevice,
    );
}

impl<D, U, V> Dispatch<ZwlrDataControlDeviceV1, U, D> for DataControlDeviceManagerState<V>
where
    D: Dispatch<ZwlrDataControlDeviceV1, U>
        + Dispatch<ZwlrDataControlOfferV1, V>
        + DataControlDeviceHandler
        + DataControlOfferHandler
        + 'static,
    U: DataControlDeviceDataExt,
    V: DataControlOfferDataExt + Default + 'static + Send + Sync,
{
    event_created_child!(D, ZwlrDataControlDeviceV1, [
        0 => (ZwlrDataControlOfferV1, V::default())
    ]);

    fn event(
        state: &mut D,
        data_device: &ZwlrDataControlDeviceV1,
        event: Event,
        data: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let data = data.data_control_device_data();
        let inner = lock!(data.inner);

        match event {
            Event::DataOffer { id } => {
                // XXX Drop done here to prevent Mutex deadlocks.S

                lock!(inner.undetermined_offers).push(id.clone());
                let data = id
                    .data::<V>()
                    .expect(ERR_WAYLAND_DATA)
                    .data_control_offer_data();
                data.init_undetermined_offer(&id);

                // Append the data offer to our list of offers.
                drop(inner);
            }
            Event::Selection { id } => {
                let mut selection_offer = lock!(inner.selection_offer);

                if let Some(offer) = id {
                    let mut undetermined = lock!(inner.undetermined_offers);
                    if let Some(i) = undetermined.iter().position(|o| o == &offer) {
                        undetermined.remove(i);
                    }
                    drop(undetermined);

                    let data = offer
                        .data::<V>()
                        .expect(ERR_WAYLAND_DATA)
                        .data_control_offer_data();
                    data.to_selection_offer();
                    // XXX Drop done here to prevent Mutex deadlocks.
                    *selection_offer = Some(offer.clone());
                    drop(selection_offer);
                    drop(inner);
                    state.selection(
                        conn,
                        qh,
                        DataControlDevice {
                            device: data_device.clone(),
                        },
                    );
                } else {
                    *selection_offer = None;
                }
            }
            Event::Finished => {
                warn!("Data control offer is no longer valid, but has not been dropped by client. This could cause clipboard issues.");
            }
            _ => {}
        }
    }
}
