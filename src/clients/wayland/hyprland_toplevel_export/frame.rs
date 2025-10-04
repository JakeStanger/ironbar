use super::manager::ToplevelManagerState;
use super::{Buffer, BufferRequest};
use crate::clients::wayland::ToplevelHandle;
use crate::lock;
use color_eyre::Result;
use std::sync::{Arc, Mutex};
use tracing::{error, warn};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols_hyprland::toplevel_export::v1::client::hyprland_toplevel_export_frame_v1::{
    Event, Flags, HyprlandToplevelExportFrameV1,
};

pub trait ToplevelFrameDataExt: Send + Sync {
    fn buffer_request(&self) -> BufferRequest;
    fn set_buffer_request(&self, buffer: BufferRequest);

    fn handle(&self) -> Option<&ToplevelHandle>;

    fn copied_first_frame(&self) -> bool;
    fn set_copied_first_frame(&self);
}

impl ToplevelFrameDataExt for ToplevelFrameData {
    fn buffer_request(&self) -> BufferRequest {
        let inner = lock!(self.inner);
        inner.buffer_request
    }

    fn set_buffer_request(&self, request: BufferRequest) {
        lock!(self.inner).buffer_request = request;
    }

    fn handle(&self) -> Option<&ToplevelHandle> {
        self.handle.as_ref()
    }

    fn copied_first_frame(&self) -> bool {
        lock!(self.inner).copied_first_frame
    }

    fn set_copied_first_frame(&self) {
        lock!(self.inner).copied_first_frame = true;
    }
}

#[derive(Default, Debug)]
pub struct ToplevelFrameData {
    pub handle: Option<ToplevelHandle>,
    pub inner: Arc<Mutex<ToplevelFrameDataInner>>,
}

impl ToplevelFrameData {}

#[derive(Debug, Default)]
pub struct ToplevelFrameDataInner {
    buffer_request: BufferRequest,
    copied_first_frame: bool,
}

pub trait ToplevelFrameHandler: Sized {
    /// Requests a new DMA-BUF is created for the provided parameters.
    fn dma_buffer(&mut self, request: BufferRequest, handle_id: usize) -> Result<Buffer>;

    /// Provides the buffer once ready.
    /// This includes the copied contents.
    fn buffer_ready(&mut self, handle: &ToplevelHandle);
}

impl<D, U> Dispatch<HyprlandToplevelExportFrameV1, U, D> for ToplevelManagerState
where
    D: Dispatch<HyprlandToplevelExportFrameV1, U> + ToplevelFrameHandler,
    U: ToplevelFrameDataExt,
{
    fn event(
        state: &mut D,
        proxy: &HyprlandToplevelExportFrameV1,
        event: Event,
        data: &U,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        match event {
            Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                data.set_buffer_request(BufferRequest { format, width, height })
            },
            Event::BufferDone => {
                let Some(handle_id) = data.handle().and_then(|h| h.info()).map(|i| i.id) else {
                    error!("Missing handle");
                    return;
                };

                match state.dma_buffer(data.buffer_request(), handle_id) {
                    Ok(buffer) => {
                        proxy.copy(&buffer.wl_buffer, !data.copied_first_frame() as i32);
                    },
                    Err(err) => { error!("failed to fetch buffer: {err:?}"); proxy.destroy() },
                }
            }
            Event::Flags { flags } => match flags {
                WEnum::Value(flags) => {
                    if flags.contains(Flags::YInvert) {
                        error!("Received unhandled YInvert transform flag");
                    }
                }
                WEnum::Unknown(_) => {
                    error!("Received unknown flags for toplevel frame");
                }
            },
            Event::Ready { .. } => {
                let handle = data.handle().unwrap();
                state.buffer_ready(handle);
                data.set_copied_first_frame();

                proxy.destroy();
            }
            Event::Failed => {
                error!("Failed to capture frame");
                proxy.destroy();
            }
            Event::Buffer { .. /* shm ignored in favour of dmabuf */ } | Event::Damage { .. } => {}
            _ => warn!("Received unhandled toplevel frame event: {:?}", event),
        }
    }
}
