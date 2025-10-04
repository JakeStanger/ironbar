use std::collections::HashMap;
use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd};
use drm::Device;
use drm::control::Device as ControlDevice;
use smithay_client_toolkit::dmabuf::{DmabufFeedback, DmabufHandler, DmabufState};
use smithay_client_toolkit::reexports::protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1;
use smithay_client_toolkit::reexports::protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1;
use tracing::{error, trace};
use udev::DeviceType;
use wayland_client::{Connection, QueueHandle};
use wayland_client::protocol::wl_buffer::WlBuffer;
use crate::clients::wayland::{Environment};
use color_eyre::eyre::OptionExt;
use color_eyre::Result;

struct Card(File);

#[allow(non_camel_case_types)]
type dev_t = u64;

impl Card {
    fn open(device: dev_t) -> Result<Self> {
        let device = udev::Device::from_devnum(DeviceType::Character, device)?;

        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        options.write(true);

        let file = device
            .devnode()
            .map(|node| options.open(node))
            .ok_or_eyre("Device has no node")??;

        Ok(Self(file))
    }
}

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for Card {}
impl ControlDevice for Card {}

impl DmabufHandler for Environment {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_feedback(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _proxy: &ZwpLinuxDmabufFeedbackV1,
        feedback: DmabufFeedback,
    ) {
        trace!("Got feedback: {feedback:?}");

        let format_table = feedback.format_table();
        let tranches = feedback.tranches();

        let mut formats = HashMap::new();

        for tranch in tranches {
            for index in &tranch.formats {
                let format = format_table[*index as usize];
                formats
                    .entry(format.format)
                    .or_insert_with(Vec::new)
                    .push(format.modifier);
            }
        }

        self.dmabuf_formats.extend(formats);

        // FIXME: This will probably break on multi-gpu setups
        //  may need to try each tranche in order
        let card = Card::open(feedback.main_device()).unwrap();

        let gbm_dev = gbm::Device::new(card.0).unwrap();
        self.gbm_device = Some(gbm_dev);
    }

    fn created(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        params: &ZwpLinuxBufferParamsV1,
        _buffer: WlBuffer,
    ) {
        trace!("created (async)");
        params.destroy();
    }

    fn failed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        params: &ZwpLinuxBufferParamsV1,
    ) {
        error!("Failed to create DMA-BUF buffer");
        params.destroy();
    }

    fn released(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _buffer: &WlBuffer) {
        trace!("DMA-BUF handle released");
    }
}
