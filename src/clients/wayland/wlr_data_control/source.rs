use smithay_client_toolkit::data_device::WritePipe;
use std::os::fd::FromRawFd;
use wayland_client::{Attached, DispatchData};
use wayland_protocols::wlr::unstable::data_control::v1::client::{
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_source_v1::{Event, ZwlrDataControlSourceV1},
};

fn data_control_source_impl<F>(
    source: &ZwlrDataControlSourceV1,
    event: Event,
    implem: &mut F,
    ddata: DispatchData,
) where
    F: FnMut(String, WritePipe, DispatchData),
{
    match event {
        Event::Send { mime_type, fd } => {
            let pipe = unsafe { FromRawFd::from_raw_fd(fd) };
            implem(mime_type, pipe, ddata);
        }
        Event::Cancelled => source.destroy(),
        _ => unreachable!(),
    }
}

pub struct DataControlSource {
    pub(crate) source: ZwlrDataControlSourceV1,
}

impl DataControlSource {
    pub fn new<F>(
        manager: &Attached<ZwlrDataControlManagerV1>,
        mime_types: Vec<String>,
        mut callback: F,
    ) -> Self
    where
        F: FnMut(String, WritePipe, DispatchData) + 'static,
    {
        let source = manager.create_data_source();

        source.quick_assign(move |source, evt, ddata| {
            data_control_source_impl(&source, evt, &mut callback, ddata);
        });

        for mime_type in mime_types {
            source.offer(mime_type);
        }

        Self {
            source: source.detach(),
        }
    }
}
