pub mod device;
pub mod manager;
pub mod offer;
pub mod source;

use self::device::{DataControlDeviceDataExt, DataControlDeviceHandler};
use self::offer::{DataControlDeviceOffer, DataControlOfferHandler, SelectionOffer};
use self::source::DataControlSourceHandler;
use super::{Client, Environment, Event, Request, Response};
use crate::channels::AsyncSenderExt;
use crate::{lock, Ironbar};
use device::DataControlDevice;
use glib::Bytes;
use nix::fcntl::{fcntl, F_GETPIPE_SZ, F_SETPIPE_SZ};
use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};
use smithay_client_toolkit::data_device_manager::WritePipe;
use smithay_client_toolkit::reexports::calloop::{PostAction, RegistrationToken};
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::sync::Arc;
use std::{fs, io};
use tokio::sync::broadcast;
use tracing::{debug, error, trace};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1;

const INTERNAL_MIME_TYPE: &str = "x-ironbar-internal";

#[derive(Debug)]
pub struct SelectionOfferItem {
    offer: SelectionOffer,
    token: Option<RegistrationToken>,
}

/// Represents a value which can be read/written
/// to/from the system clipboard and surrounding metadata.
///
/// Can be cheaply cloned.
#[derive(Debug, Clone, Eq)]
pub struct ClipboardItem {
    pub id: usize,
    pub value: Arc<ClipboardValue>,
    pub mime_type: Arc<str>,
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

impl Client {
    /// Gets the current clipboard item,
    /// if this exists and Ironbar has record of it.
    pub fn clipboard_item(&self) -> Option<ClipboardItem> {
        match self.send_request(Request::ClipboardItem) {
            Response::ClipboardItem(item) => item,
            _ => unreachable!(),
        }
    }

    /// Copies the provided value to the system clipboard.
    pub fn copy_to_clipboard(&self, item: ClipboardItem) {
        match self.send_request(Request::CopyToClipboard(item)) {
            Response::Ok => (),
            _ => unreachable!(),
        }
    }

    /// Subscribes to the system clipboard,
    /// receiving all new copied items.
    pub fn subscribe_clipboard(&self) -> broadcast::Receiver<ClipboardItem> {
        self.clipboard_channel.0.subscribe()
    }
}

impl Environment {
    /// Creates a new copy/paste source on the
    /// seat's data control device.
    ///
    /// This provides it as an offer,
    /// which the compositor will then treat as the current copied value.
    pub fn copy_to_clipboard(&mut self, item: ClipboardItem) {
        debug!("Copying item to clipboard: {item:?}");

        let seat = self.default_seat();
        let Some(device) = self
            .data_control_devices
            .iter()
            .find(|entry| entry.seat == seat)
        else {
            return;
        };

        let source = self
            .data_control_device_manager_state
            .create_copy_paste_source(&self.queue_handle, [INTERNAL_MIME_TYPE, &item.mime_type]);

        source.set_selection(&device.device);
        self.copy_paste_sources.push(source);

        lock!(self.clipboard).replace(item);
    }

    /// Reads an offer file handle into a new `ClipboardItem`.
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

                debug!("Read bytes: {}", bytes.len());

                let bytes = Bytes::from(&bytes);

                ClipboardValue::Image(bytes)
            }
        };

        Ok(ClipboardItem {
            id: Ironbar::unique_id(),
            value: Arc::new(value),
            mime_type: mime_type.value.clone().into(),
        })
    }
}

impl DataControlDeviceHandler for Environment {
    /// Called when an offer for a new value is received
    /// (ie something has copied to the clipboard)
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

            // clear prev
            let Some(mime_type) = MimeType::parse_multiple(&mime_types) else {
                lock!(self.clipboard).take();
                // send an event so the clipboard module is aware it's changed
                self.event_tx.send_spawn(Event::Clipboard(ClipboardItem {
                    id: usize::MAX,
                    mime_type: String::new().into(),
                    value: Arc::new(ClipboardValue::Other),
                }));
                return;
            };

            debug!("Receiving mime type: {}", mime_type.value);

            if let Ok(read_pipe) = cur_offer.offer.receive(mime_type.value.clone()) {
                let offer_clone = cur_offer.offer.clone();

                let tx = self.event_tx.clone();
                let clipboard = self.clipboard.clone();

                let token =
                    self.loop_handle
                        .insert_source(read_pipe, move |(), file, state| unsafe {
                            let item = state
                                .selection_offers
                                .iter()
                                .position(|o| o.offer == offer_clone)
                                .map(|p| state.selection_offers.remove(p))
                                .expect("Failed to find selection offer item");

                            match Self::read_file(&mime_type, file.get_mut()) {
                                Ok(item) => {
                                    lock!(clipboard).replace(item.clone());

                                    tx.send_spawn(Event::Clipboard(item));
                                }
                                Err(err) => error!("{err:?}"),
                            }

                            state
                                .loop_handle
                                .remove(item.token.expect("Missing item token"));

                            PostAction::Remove
                        });

                match token {
                    Ok(token) => {
                        cur_offer.token.replace(token);
                    }
                    Err(err) => error!("Failed to insert read pipe event: {err:?}"),
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
        trace!("Handler received offer");
    }
}

impl DataControlSourceHandler for Environment {
    // fn accept_mime(
    //     &mut self,
    //     _conn: &Connection,
    //     _qh: &QueueHandle<Self>,
    //     _source: &ZwlrDataControlSourceV1,
    //     mime: Option<String>,
    // ) {
    //     debug!("Accepted mime type: {mime:?}");
    // }

    /// Writes the current clipboard item to 'paste' it
    /// upon request from a compositor client.
    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &ZwlrDataControlSourceV1,
        mime: String,
        write_pipe: WritePipe,
    ) {
        debug!("Handler received source send request event ({mime})");

        if let Some(item) = lock!(self.clipboard).clone() {
            let fd = OwnedFd::from(write_pipe);
            if self
                .copy_paste_sources
                .iter_mut()
                .any(|s| s.inner() == source && MimeType::parse(&mime).is_some())
            {
                trace!("Source found, writing to file");

                let mut bytes = match item.value.as_ref() {
                    ClipboardValue::Text(text) => text.as_bytes(),
                    ClipboardValue::Image(bytes) => bytes.as_ref(),
                    ClipboardValue::Other => panic!(
                        "{:?}",
                        io::Error::new(ErrorKind::Other, "Attempted to copy unsupported mime type")
                    ),
                };

                let pipe_size = set_pipe_size(fd.as_raw_fd(), bytes.len())
                    .expect("Failed to increase pipe size");
                let mut file = File::from(fd.try_clone().expect("to be able to clone"));

                debug!("Writing {} bytes", bytes.len());

                let mut events = (0..16).map(|_| EpollEvent::empty()).collect::<Vec<_>>();
                let epoll_event = EpollEvent::new(EpollFlags::EPOLLOUT, 0);

                let epoll_fd =
                    Epoll::new(EpollCreateFlags::empty()).expect("to get valid file descriptor");
                epoll_fd
                    .add(fd, epoll_event)
                    .expect("to send valid epoll operation");

                let timeout = EpollTimeout::from(100u16);
                while !bytes.is_empty() {
                    let chunk = &bytes[..min(pipe_size as usize, bytes.len())];

                    epoll_fd
                        .wait(&mut events, timeout)
                        .expect("Failed to wait to epoll");

                    match file.write(chunk) {
                        Ok(written) => {
                            trace!("Wrote {} bytes ({} remain)", written, bytes.len());
                            bytes = &bytes[written..];
                        }
                        Err(err) => {
                            error!("{err:?}");
                            break;
                        }
                    }
                }

                debug!("Done writing");
            } else {
                error!("Failed to find source");
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

/// Attempts to increase the fd pipe size to the requested number of bytes.
/// The kernel will automatically round this up to the nearest page size.
/// If the requested size is larger than the kernel max (normally 1MB),
/// it will be clamped at this.
///
/// Returns the new size if succeeded.
fn set_pipe_size(fd: RawFd, size: usize) -> io::Result<i32> {
    // clamp size at kernel max
    let max_pipe_size = fs::read_to_string("/proc/sys/fs/pipe-max-size")
        .expect("Failed to find pipe-max-size virtual kernel file")
        .trim()
        .parse::<usize>()
        .expect("Failed to parse pipe-max-size contents");

    let size = min(size, max_pipe_size);

    let curr_size = fcntl(fd, F_GETPIPE_SZ)? as usize;

    trace!("Current pipe size: {curr_size}");

    let new_size = if size > curr_size {
        trace!("Requesting pipe size increase to (at least): {size}");

        let res = fcntl(fd, F_SETPIPE_SZ(size as i32))?;
        trace!("New pipe size: {res}");

        if res < size as i32 {
            return Err(io::Error::last_os_error());
        }

        res
    } else {
        size as i32
    };

    Ok(new_size)
}
