pub mod device;
pub mod manager;
pub mod offer;
pub mod source;

use super::Env;
use crate::clients::wayland::DData;
use crate::send;
use color_eyre::Report;
use device::{DataControlDevice, DataControlDeviceEvent};
use glib::Bytes;
use manager::{DataControlDeviceHandling, DataControlDeviceStatusListener};
use smithay_client_toolkit::data_device::WritePipe;
use smithay_client_toolkit::environment::Environment;
use smithay_client_toolkit::reexports::calloop::LoopHandle;
use smithay_client_toolkit::MissingGlobal;
use source::DataControlSource;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::sync::broadcast;
use tracing::{debug, error, trace};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::DispatchData;

static COUNTER: AtomicUsize = AtomicUsize::new(1);

const INTERNAL_MIME_TYPE: &str = "x-ironbar-internal";

fn get_id() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardValue {
    Text(String),
    Image(Bytes),
    Other,
}

impl DataControlDeviceHandling for Env {
    fn listen<F>(&mut self, f: F) -> DataControlDeviceStatusListener
    where
        F: FnMut(WlSeat, DataControlDeviceEvent, DispatchData) + 'static,
    {
        self.data_control_device.listen(f)
    }

    fn with_data_control_device<F>(&self, seat: &WlSeat, f: F) -> Result<(), MissingGlobal>
    where
        F: FnOnce(&DataControlDevice),
    {
        self.data_control_device.with_data_control_device(seat, f)
    }

    fn create_source<F>(&self, mime_types: Vec<String>, callback: F) -> Option<DataControlSource>
    where
        F: FnMut(String, WritePipe, DispatchData) + 'static,
    {
        self.data_control_device.create_source(mime_types, callback)
    }
}

pub fn copy_to_clipboard<E>(
    env: &Environment<E>,
    seat: &WlSeat,
    item: &ClipboardItem,
) -> Result<(), MissingGlobal>
where
    E: DataControlDeviceHandling,
{
    debug!("Copying item with id {} [{}]", item.id, item.mime_type);
    trace!("Copying: {item:?}");

    let item = item.clone();

    env.with_inner(|env| {
        let mime_types = vec![INTERNAL_MIME_TYPE.to_string(), item.mime_type];
        let source = env.create_source(mime_types, move |mime_type, mut pipe, _ddata| {
            debug!(
                "Triggering source callback for item with id {} [{}]",
                item.id, mime_type
            );

            // FIXME: Not working for large (buffered) values in xwayland
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

            if let Err(err) = pipe.write_all(bytes) {
                error!("{err:?}");
            }
        });

        env.with_data_control_device(seat, |device| device.set_selection(&source))
    })
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
    fn parse(mime_types: &[String]) -> Option<Self> {
        mime_types
            .iter()
            .map(|s| s.to_lowercase())
            .find_map(|mime_type| match mime_type.as_str() {
                "text"
                | "string"
                | "utf8_string"
                | "text/plain"
                | "text/plain;charset=utf-8"
                | "text/plain;charset=iso-8859-1"
                | "text/plain;charset=us-ascii"
                | "text/plain;charset=unicode" => Some(Self {
                    value: mime_type,
                    category: MimeTypeCategory::Text,
                }),
                "image/png" | "image/jpg" | "image/jpeg" | "image/tiff" | "image/bmp"
                | "image/x-bmp" | "image/icon" => Some(Self {
                    value: mime_type,
                    category: MimeTypeCategory::Image,
                }),
                _ => None,
            })
    }
}

pub fn receive_offer(
    event: DataControlDeviceEvent,
    handle: &LoopHandle<DData>,
    tx: broadcast::Sender<Arc<ClipboardItem>>,
    mut ddata: DispatchData,
) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Could not get epoch, system time is probably very wrong")
        .as_nanos();

    let offer = event.0;

    let ddata = ddata
        .get::<DData>()
        .expect("Expected dispatch data to exist");

    let handle2 = handle.clone();

    let res = offer.with_mime_types(|mime_types| {
        debug!("Offer mime types: {mime_types:?}");

        if mime_types.contains(&INTERNAL_MIME_TYPE.to_string()) {
            debug!("Skipping value provided by bar");
            return Ok(());
        }

        let mime_type = MimeType::parse(mime_types);
        debug!("Detected mime type: {mime_type:?}");

        match mime_type {
            Some(mime_type) => {
                debug!("[{timestamp}] Sending clipboard read request ({mime_type:?})");
                let read_pipe = offer.receive(mime_type.value.clone())?;
                let source = handle.insert_source(read_pipe, move |(), file, ddata| {
                    debug!(
                        "[{timestamp}] Reading clipboard contents ({:?})",
                        &mime_type.category
                    );
                    match read_file(&mime_type, file) {
                        Ok(item) => {
                            send!(tx, Arc::new(item));
                        }
                        Err(err) => error!("{err:?}"),
                    }

                    if let Some(src) = ddata.offer_tokens.remove(&timestamp) {
                        handle2.remove(src);
                    }
                })?;

                ddata.offer_tokens.insert(timestamp, source);
            }
            None => {
                // send an event so the clipboard module is aware it's changed
                send!(
                    tx,
                    Arc::new(ClipboardItem {
                        id: usize::MAX,
                        mime_type: String::new(),
                        value: ClipboardValue::Other
                    })
                );
            }
        }

        Ok::<(), Report>(())
    });

    if let Err(err) = res {
        error!("{err:?}");
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
