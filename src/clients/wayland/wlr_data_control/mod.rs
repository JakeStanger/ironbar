pub mod device;
pub mod manager;
pub mod offer;
pub mod source;

use self::device::{DataControlDeviceDataExt, DataControlDeviceHandler};
use self::offer::{DataControlDeviceOffer, DataControlOfferHandler, SelectionOffer};
use self::source::DataControlSourceHandler;
use crate::clients::wayland::Environment;
use crate::{lock, send};
use device::DataControlDevice;
use glib::Bytes;
use smithay_client_toolkit::data_device_manager::WritePipe;
use smithay_client_toolkit::reexports::calloop::RegistrationToken;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, error};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1;

static COUNTER: AtomicUsize = AtomicUsize::new(1);

const INTERNAL_MIME_TYPE: &str = "x-ironbar-internal";

fn get_id() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub struct SelectionOfferItem {
    offer: SelectionOffer,
    token: Option<RegistrationToken>,
}

#[derive(Debug, Clone, Eq)]
pub struct ClipboardItem {
    pub id: usize,
    pub value: ClipboardValue,
    pub mime_type: String,
}

impl PartialEq<Self> for ClipboardItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum ClipboardValue {
    Text(String),
    Image(Bytes),
    Other,
}

impl Debug for ClipboardValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Text(text) => text.clone(),
                Self::Image(bytes) => {
                    format!("[{} Bytes]", bytes.len())
                }
                Self::Other => "[Unknown]".to_string(),
            }
        )
    }
}

#[derive(Debug)]
struct MimeType {
    value: String,
    category: MimeTypeCategory,
}

#[derive(Debug)]
enum MimeTypeCategory {
    Text,
    Image,
}

impl MimeType {
    fn parse(mime_type: &str) -> Option<Self> {
        match mime_type.to_lowercase().as_str() {
            "text"
            | "string"
            | "utf8_string"
            | "text/plain"
            | "text/plain;charset=utf-8"
            | "text/plain;charset=iso-8859-1"
            | "text/plain;charset=us-ascii"
            | "text/plain;charset=unicode" => Some(Self {
                value: mime_type.to_string(),
                category: MimeTypeCategory::Text,
            }),
            "image/png" | "image/jpg" | "image/jpeg" | "image/tiff" | "image/bmp"
            | "image/x-bmp" | "image/icon" => Some(Self {
                value: mime_type.to_string(),
                category: MimeTypeCategory::Image,
            }),
            _ => None,
        }
    }

    fn parse_multiple(mime_types: &[String]) -> Option<Self> {
        mime_types.iter().find_map(|mime| Self::parse(mime))
    }
}

impl Environment {
    pub fn copy_to_clipboard(&mut self, item: Arc<ClipboardItem>, qh: &QueueHandle<Self>) {
        debug!("Copying item to clipboard");

        // TODO: Proper device tracking
        let device = self.data_control_devices.first();
        if let Some(device) = device {
            let source = self
                .data_control_device_manager_state
                .create_copy_paste_source(qh, [item.mime_type.as_str()]);

            source.set_selection(&device.device);
            self.copy_paste_sources.push(source);

            lock!(self.clipboard).replace(item);
        }
    }

    fn read_file(mime_type: &MimeType, file: &mut File) -> io::Result<ClipboardItem> {
        let value = match mime_type.category {
            MimeTypeCategory::Text => {
                let mut txt = String::new();
                file.read_to_string(&mut txt)?;

                ClipboardValue::Text(txt)
            }
            MimeTypeCategory::Image => {
                let mut bytes = vec![];
                file.read_to_end(&mut bytes)?;
                let bytes = Bytes::from(&bytes);

                ClipboardValue::Image(bytes)
            }
        };

        Ok(ClipboardItem {
            id: get_id(),
            value,
            mime_type: mime_type.value.clone(),
        })
    }
}

impl DataControlDeviceHandler for Environment {
    fn selection(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        data_device: DataControlDevice,
    ) {
        debug!("Handler received selection event");

        let mime_types = data_device.selection_mime_types();

        if mime_types.contains(&INTERNAL_MIME_TYPE.to_string()) {
            return;
        }

        if let Some(offer) = data_device.selection_offer() {
            self.selection_offers
                .push(SelectionOfferItem { offer, token: None });

            let cur_offer = self
                .selection_offers
                .last_mut()
                .expect("Failed to get current offer");

            let Some(mime_type) = MimeType::parse_multiple(&mime_types) else {
                lock!(self.clipboard).take();
                // send an event so the clipboard module is aware it's changed
                send!(
                    self.clipboard_tx,
                    Arc::new(ClipboardItem {
                        id: usize::MAX,
                        mime_type: String::new(),
                        value: ClipboardValue::Other
                    })
                );
                return;
            };

            if let Ok(read_pipe) = cur_offer.offer.receive(mime_type.value.clone()) {
                let offer_clone = cur_offer.offer.clone();

                let tx = self.clipboard_tx.clone();
                let clipboard = self.clipboard.clone();

                let token = self
                    .loop_handle
                    .insert_source(read_pipe, move |_, file, state| {
                        let item = state
                            .selection_offers
                            .iter()
                            .position(|o| o.offer == offer_clone)
                            .map(|p| state.selection_offers.remove(p))
                            .expect("Failed to find selection offer item");

                        match Self::read_file(&mime_type, file) {
                            Ok(item) => {
                                let item = Arc::new(item);
                                lock!(clipboard).replace(item.clone());
                                send!(tx, item);
                            }
                            Err(err) => error!("{err:?}"),
                        }

                        state
                            .loop_handle
                            .remove(item.token.expect("Missing item token"));
                    });

                match token {
                    Ok(token) => {
                        cur_offer.token.replace(token);
                    }
                    Err(err) => error!("{err:?}"),
                }
            }
        }
    }
}

impl DataControlOfferHandler for Environment {
    fn offer(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut DataControlDeviceOffer,
        _mime_type: String,
    ) {
        debug!("Handler received offer");
    }
}

impl DataControlSourceHandler for Environment {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &ZwlrDataControlSourceV1,
        mime: Option<String>,
    ) {
        debug!("Accepted mime type: {mime:?}");
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &ZwlrDataControlSourceV1,
        mime: String,
        write_pipe: WritePipe,
    ) {
        debug!("Handler received source send request event");

        if let Some(item) = lock!(self.clipboard).clone() {
            let fd = OwnedFd::from(write_pipe);
            if let Some(_source) = self
                .copy_paste_sources
                .iter_mut()
                .find(|s| s.inner() == source && MimeType::parse(&mime).is_some())
            {
                let mut file = File::from(fd);

                // FIXME: Not working for large (buffered) values in xwayland
                //  Might be something strange going on with byte count?
                let bytes = match &item.value {
                    ClipboardValue::Text(text) => text.as_bytes(),
                    ClipboardValue::Image(bytes) => bytes.as_ref(),
                    ClipboardValue::Other => panic!(
                        "{:?}",
                        io::Error::new(
                            io::ErrorKind::Other,
                            "Attempted to copy unsupported mime type",
                        )
                    ),
                };

                if let Err(err) = file.write_all(bytes) {
                    error!("{err:?}");
                }
            }
        }
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &ZwlrDataControlSourceV1,
    ) {
        debug!("Handler received source cancelled event");

        self.copy_paste_sources
            .iter()
            .position(|s| s.inner() == source)
            .map(|pos| self.copy_paste_sources.remove(pos));
        source.destroy();
    }
}
