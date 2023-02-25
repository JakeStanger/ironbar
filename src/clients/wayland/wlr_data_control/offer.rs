use crate::lock;
use nix::fcntl::OFlag;
use nix::unistd::{close, pipe2};
use smithay_client_toolkit::data_device::ReadPipe;
use std::io;
use std::os::fd::FromRawFd;
use std::sync::{Arc, Mutex};
use tracing::warn;
use wayland_client::Main;
use wayland_protocols::wlr::unstable::data_control::v1::client::zwlr_data_control_offer_v1::{
    Event, ZwlrDataControlOfferV1,
};

#[derive(Debug, Clone)]
struct Inner {
    mime_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DataControlOffer {
    inner: Arc<Mutex<Inner>>,
    pub(crate) offer: ZwlrDataControlOfferV1,
}

impl DataControlOffer {
    pub(crate) fn new(offer: &Main<ZwlrDataControlOfferV1>) -> Self {
        let inner = Arc::new(Mutex::new(Inner {
            mime_types: Vec::new(),
        }));

        {
            let inner = inner.clone();

            offer.quick_assign(move |_, event, _| {
                let mut inner = lock!(inner);
                if let Event::Offer { mime_type } = event {
                    inner.mime_types.push(mime_type);
                }
            });
        }

        Self {
            offer: offer.detach(),
            inner,
        }
    }

    pub fn with_mime_types<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&[String]) -> T,
    {
        let inner = lock!(self.inner);
        f(&inner.mime_types)
    }

    pub fn receive(&self, mime_type: String) -> io::Result<ReadPipe> {
        // create a pipe
        let (readfd, writefd) = pipe2(OFlag::O_CLOEXEC)?;

        self.offer.receive(mime_type, writefd);

        if let Err(err) = close(writefd) {
            warn!("Failed to close write pipe: {}", err);
        }

        Ok(unsafe { FromRawFd::from_raw_fd(readfd) })
    }
}

impl Drop for DataControlOffer {
    fn drop(&mut self) {
        self.offer.destroy();
    }
}
