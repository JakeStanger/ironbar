use super::device::DataControlDevice;
use super::manager::DataControlDeviceManagerState;
use color_eyre::Result;
use smithay_client_toolkit::data_device_manager::WritePipe;
use tracing::error;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::{
    Event, ZwlrDataControlSourceV1,
};

#[derive(Debug, Default)]
pub struct DataControlSourceData {}

pub trait DataControlSourceDataExt: Send + Sync {
    // fn data_source_data(&self) -> &DataControlSourceData;
}

impl DataControlSourceDataExt for DataControlSourceData {
    // fn data_source_data(&self) -> &DataControlSourceData {
    //     self
    // }
}

/// Handler trait for `DataSource` events.
///
/// The functions defined in this trait are called as `DataSource` events are received from the compositor.
pub trait DataControlSourceHandler: Sized {
    // /// This may be called multiple times, once for each accepted mime type from the destination, if any.
    // fn accept_mime(
    //     &mut self,
    //     conn: &Connection,
    //     qh: &QueueHandle<Self>,
    //     source: &ZwlrDataControlSourceV1,
    //     mime: Option<String>,
    // );

    /// The client has requested the data for this source to be sent.
    /// Send the data, then close the fd.
    fn send_request(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        source: &ZwlrDataControlSourceV1,
        mime: String,
        fd: WritePipe,
    ) -> Result<()>;

    /// The data source is no longer valid
    /// Cleanup & destroy this resource
    fn cancelled(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        source: &ZwlrDataControlSourceV1,
    );
}

impl<D, U> Dispatch<ZwlrDataControlSourceV1, U, D> for DataControlDeviceManagerState
where
    D: Dispatch<ZwlrDataControlSourceV1, U> + DataControlSourceHandler,
    U: DataControlSourceDataExt,
{
    fn event(
        state: &mut D,
        source: &ZwlrDataControlSourceV1,
        event: <ZwlrDataControlSourceV1 as Proxy>::Event,
        _data: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            Event::Send { mime_type, fd } => {
                if let Err(err) = state.send_request(conn, qh, source, mime_type, fd.into()) {
                    error!("{err:#}");
                }
            }
            Event::Cancelled => {
                state.cancelled(conn, qh, source);
            }
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CopyPasteSource {
    pub(crate) inner: ZwlrDataControlSourceV1,
}

impl CopyPasteSource {
    /// Set the selection of the provided data device as a response to the event with with provided serial.
    pub fn set_selection(&self, device: &DataControlDevice) {
        device.device.set_selection(Some(&self.inner));
    }

    pub const fn inner(&self) -> &ZwlrDataControlSourceV1 {
        &self.inner
    }
}

impl Drop for CopyPasteSource {
    fn drop(&mut self) {
        self.inner.destroy();
    }
}
