use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::sync::Arc;
use gbm::{BufferObjectFlags, Modifier};
use smithay_client_toolkit::reexports::protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::Flags;
use wayland_client::protocol::wl_buffer::WlBuffer;
use super::{Client, Environment, Event, Request, Response, ToplevelEvent, ToplevelHandle};
use super::hyprland_toplevel_export::manager::ToplevelManagerHandler;
use crate::clients::wayland::hyprland_toplevel_export::frame::{ToplevelFrameData, ToplevelFrameHandler};
use color_eyre::{Report, Result};
use drm::buffer::DrmModifier;
use gtk::gdk::{DmabufTextureBuilder, Texture};
use log::error;
use crate::channels::AsyncSenderExt;

pub mod frame;
pub mod manager;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct BufferRequest {
    width: u32,
    height: u32,
    format: u32,
}

#[derive(Debug)]
pub struct Plane {
    fd: OwnedFd,
    offset: u32,
    stride: u32,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub width: u32,
    pub height: u32,
    format: u32,
    modifier: DrmModifier,
    wl_buffer: WlBuffer,
    planes: Arc<Vec<Plane>>,
}

impl Buffer {
    fn is_compatible(&self, req: BufferRequest) -> bool {
        self.width == req.width && self.height == req.height && self.format == req.format
    }
}

impl TryFrom<Buffer> for Texture {
    type Error = glib::Error;

    fn try_from(buffer: Buffer) -> std::result::Result<Self, Self::Error> {
        let mut texture_builder = DmabufTextureBuilder::new()
            .set_width(buffer.width)
            .set_height(buffer.height)
            .set_fourcc(buffer.format)
            .set_modifier(buffer.modifier.into())
            .set_n_planes(buffer.planes.len() as u32);

        for (i, plane) in buffer.planes.iter().enumerate() {
            texture_builder = unsafe { texture_builder.set_fd(i as u32, plane.fd.as_raw_fd()) }
                .set_offset(i as u32, plane.offset)
                .set_stride(i as u32, plane.stride);
        }

        let texture = unsafe { texture_builder.build() }?;
        Ok(texture)
    }
}

// TODO: We should implement double-buffering ideally
//  but a single buffer for each preview seems to work well enough for now?
// #[derive(Debug, Default)]
// pub struct DoubleBuffer {
//     swapped: Cell<bool>,
//     buffers: [RefCell<Option<Buffer>>; 2],
// }
//
// impl DoubleBuffer {
//     fn swap(&self) {
//         self.swapped.replace(!self.swapped.get());
//     }
//
//     pub fn current(&self) -> Option<Buffer> {
//         self.buffers[self.swapped.get() as usize].borrow().clone()
//     }
//
//     fn next(&self) -> Option<Buffer> {
//         self.buffers[!self.swapped.get() as usize].borrow().clone()
//     }
//
//     fn set_next(&self, buffer: Buffer) {
//         self.buffers[!self.swapped.get() as usize]
//             .borrow_mut()
//             .replace(buffer);
//     }
// }

impl Client {
    /// Gets the image buffer for a toplevel.
    pub fn toplevel_buffer(&self, toplevel_id: usize) -> Option<Buffer> {
        match self.send_request(Request::ToplevelBuffer(toplevel_id)) {
            Response::ToplevelBuffer(buffer) => buffer,
            _ => unreachable!(),
        }
    }

    /// Requests a new frame is sent to the buffer for a toplevel.
    pub fn toplevel_buffer_update(&self, toplevel_id: usize) {
        match self.send_request(Request::ToplevelBufferUpdate(toplevel_id)) {
            Response::Ok => (),
            _ => unreachable!(),
        }
    }
}

impl ToplevelManagerHandler for Environment {
    fn capture(&self, handle: &ToplevelHandle) {
        let Some(state) = &self.hyprland_toplevel_export_manager_state else {
            return;
        };

        let data = ToplevelFrameData {
            handle: Some(handle.clone()),
            ..ToplevelFrameData::default()
        };

        state.capture(&handle.handle, &self.queue_handle, data);
    }
}

impl ToplevelFrameHandler for Environment {
    fn dma_buffer(&mut self, request: BufferRequest, handle_id: usize) -> Result<Buffer> {
        if let Some(buffer) = self.toplevel_buffers.get(&handle_id)
            && buffer.is_compatible(request)
        {
            return Ok(buffer.clone());
        }

        let width = request.width;
        let height = request.height;
        let format = request.format;

        let fmt = gbm::Format::try_from(format)?;

        let Some(modifiers) = self.dmabuf_formats.get(&format) else {
            return Err(Report::msg(format!("format {format} not supported")));
        };

        let modifiers = modifiers.iter().map(|&m| Modifier::from(m));

        let buffer_object = self
            .gbm_device
            .as_ref()
            .expect("gbm device should be initialised")
            .create_buffer_object_with_modifiers2::<()>(
                width,
                height,
                fmt,
                modifiers,
                BufferObjectFlags::RENDERING,
            )?;

        let modifier = buffer_object.modifier();

        let plane_count = buffer_object.plane_count();
        let mut planes = Vec::with_capacity(plane_count as usize);

        let params = self.dmabuf_state.create_params(&self.queue_handle)?;

        for i in 0..plane_count {
            let fd = buffer_object.fd_for_plane(i as i32)?;
            let offset = buffer_object.offset(i as i32);
            let stride = buffer_object.stride_for_plane(i as i32);

            params.add(fd.as_fd(), i, offset, stride, modifier.into());

            planes.push(Plane { fd, offset, stride });
        }

        let (wl_buffer, params) = params.create_immed(
            width as i32,
            height as i32,
            format,
            Flags::empty(),
            &self.queue_handle,
        );

        params.destroy(); // required due to immediate creation

        let planes = Arc::new(planes);

        let buffer = Buffer {
            width,
            height,
            format,
            modifier,
            wl_buffer,
            planes,
        };

        self.toplevel_buffers.insert(handle_id, buffer.clone());

        Ok(buffer)
    }

    fn buffer_ready(&mut self, handle: &ToplevelHandle) {
        let Some(info) = handle.info() else {
            error!("toplevel missing info");
            return;
        };

        let Some(buffer) = self.toplevel_buffers.get(&info.id).cloned() else {
            error!("missing buffer");
            return;
        };

        self.event_tx
            .send_spawn(Event::Toplevel(ToplevelEvent::Buffer(info, buffer)));
    }
}
