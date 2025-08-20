use super::manager::DataControlDeviceManagerState;
use crate::lock;
use rustix::pipe::{PipeFlags, pipe_with};
use smithay_client_toolkit::data_device_manager::data_offer::DataOfferError;
use std::ops::DerefMut;
use std::os::fd::AsFd;
use std::sync::{Arc, Mutex};
use tokio::net::unix::pipe::Receiver;
use tracing::trace;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::{
    Event, ZwlrDataControlOfferV1,
};

#[derive(Debug, Clone)]
pub struct UndeterminedOffer {
    pub(crate) data_offer: Option<ZwlrDataControlOfferV1>,
}

impl PartialEq for UndeterminedOffer {
    fn eq(&self, other: &Self) -> bool {
        self.data_offer == other.data_offer
    }
}

#[derive(Debug, Clone)]
pub struct SelectionOffer {
    pub data_offer: ZwlrDataControlOfferV1,
}

impl PartialEq for SelectionOffer {
    fn eq(&self, other: &Self) -> bool {
        self.data_offer == other.data_offer
    }
}

impl SelectionOffer {
    pub fn receive(&self, mime_type: String) -> Result<Receiver, DataOfferError> {
        receive(&self.data_offer, mime_type).map_err(DataOfferError::Io)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataControlDeviceOffer {
    Selection(SelectionOffer),
    Undetermined(UndeterminedOffer),
}

impl Default for DataControlDeviceOffer {
    fn default() -> Self {
        Self::Undetermined(UndeterminedOffer { data_offer: None })
    }
}

#[derive(Debug, Default)]
pub struct DataControlOfferData {
    pub(crate) inner: Arc<Mutex<DataControlDeviceOfferInner>>,
}

#[derive(Debug, Default)]
pub struct DataControlDeviceOfferInner {
    pub(crate) offer: DataControlDeviceOffer,
    pub(crate) mime_types: Vec<String>,
}

impl DataControlOfferData {
    pub(crate) fn push_mime_type(&self, mime_type: String) {
        lock!(self.inner).mime_types.push(mime_type);
    }

    pub(crate) fn to_selection_offer(&self) {
        let mut inner = lock!(self.inner);
        match &mut inner.deref_mut().offer {
            DataControlDeviceOffer::Selection(_) => {}
            DataControlDeviceOffer::Undetermined(o) => {
                inner.offer = DataControlDeviceOffer::Selection(SelectionOffer {
                    data_offer: o.data_offer.clone().expect("Missing current data offer"),
                });
            }
        }
    }

    pub(crate) fn init_undetermined_offer(&self, offer: &ZwlrDataControlOfferV1) {
        let mut inner = lock!(self.inner);
        match &mut inner.deref_mut().offer {
            DataControlDeviceOffer::Selection(_) => {
                inner.offer = DataControlDeviceOffer::Undetermined(UndeterminedOffer {
                    data_offer: Some(offer.clone()),
                });
            }
            DataControlDeviceOffer::Undetermined(o) => {
                o.data_offer = Some(offer.clone());
            }
        }
    }
}

pub trait DataControlOfferDataExt {
    fn data_control_offer_data(&self) -> &DataControlOfferData;
    fn mime_types(&self) -> Vec<String>;
    fn as_selection_offer(&self) -> Option<SelectionOffer>;
}

impl DataControlOfferDataExt for DataControlOfferData {
    fn data_control_offer_data(&self) -> &DataControlOfferData {
        self
    }

    fn mime_types(&self) -> Vec<String> {
        lock!(self.inner).mime_types.clone()
    }

    fn as_selection_offer(&self) -> Option<SelectionOffer> {
        match &lock!(self.inner).offer {
            DataControlDeviceOffer::Selection(o) => Some(o.clone()),
            DataControlDeviceOffer::Undetermined(_) => None,
        }
    }
}

/// Handler trait for `DataOffer` events.
///
/// The functions defined in this trait are called as `DataOffer` events are received from the compositor.
pub trait DataControlOfferHandler: Sized {
    // Called for each mime type the data offer advertises.
    fn offer(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        offer: &mut DataControlDeviceOffer,
        mime_type: String,
    );
}

impl<D, U> Dispatch<ZwlrDataControlOfferV1, U, D> for DataControlDeviceManagerState
where
    D: Dispatch<ZwlrDataControlOfferV1, U> + DataControlOfferHandler,
    U: DataControlOfferDataExt,
{
    fn event(
        state: &mut D,
        _offer: &ZwlrDataControlOfferV1,
        event: <ZwlrDataControlOfferV1 as Proxy>::Event,
        data: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let data = data.data_control_offer_data();

        if let Event::Offer { mime_type } = event {
            trace!("Adding new offer with type '{mime_type}'");
            data.push_mime_type(mime_type.clone());
            state.offer(conn, qh, &mut lock!(data.inner).offer, mime_type);
        }
    }
}

/// Request to receive the data of a given mime type.
///
/// You can do this several times, as a reaction to motion of
/// the dnd cursor, or to inspect the data in order to choose your
/// response.
///
/// Note that you should *not* read the contents right away in a
/// blocking way, as you may deadlock your application doing so.
/// At least make sure you flush your events to the server before
/// doing so.
///
/// Fails if too many file descriptors were already open and a pipe
/// could not be created.
pub fn receive(offer: &ZwlrDataControlOfferV1, mime_type: String) -> std::io::Result<Receiver> {
    // create a pipe
    let (readfd, writefd) = pipe_with(PipeFlags::CLOEXEC)?;

    offer.receive(mime_type, writefd.as_fd());

    Receiver::from_owned_fd(readfd)
}
