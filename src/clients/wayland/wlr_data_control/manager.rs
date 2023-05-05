use super::device::{DataControlDevice, DataControlDeviceData, DataControlDeviceDataExt};
use super::offer::DataControlOfferData;
use super::source::{CopyPasteSource, DataControlSourceData, DataControlSourceDataExt};
use smithay_client_toolkit::error::GlobalError;
use smithay_client_toolkit::globals::{GlobalData, ProvidesBoundGlobal};
use std::marker::PhantomData;
use tracing::debug;
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_source_v1::ZwlrDataControlSourceV1,
};

pub struct DataControlDeviceManagerState<V = DataControlOfferData> {
    manager: ZwlrDataControlManagerV1,
    _phantom: PhantomData<V>,
}

impl DataControlDeviceManagerState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<ZwlrDataControlManagerV1, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=2, GlobalData)?;
        debug!("Bound to ZwlDataControlManagerV1 global");
        Ok(Self {
            manager,
            _phantom: PhantomData,
        })
    }

    /// creates a data source for copy paste
    pub fn create_copy_paste_source<'s, D, I>(
        &self,
        qh: &QueueHandle<D>,
        mime_types: I,
    ) -> CopyPasteSource
    where
        D: Dispatch<ZwlrDataControlSourceV1, DataControlSourceData> + 'static,
        I: IntoIterator<Item = &'s str>,
    {
        CopyPasteSource {
            inner: self.create_data_control_source(qh, mime_types),
        }
    }

    /// creates a data source
    fn create_data_control_source<'s, D, I>(
        &self,
        qh: &QueueHandle<D>,
        mime_types: I,
    ) -> ZwlrDataControlSourceV1
    where
        D: Dispatch<ZwlrDataControlSourceV1, DataControlSourceData> + 'static,
        I: IntoIterator<Item = &'s str>,
    {
        let source =
            self.create_data_control_source_with_data(qh, DataControlSourceData::default());

        for mime in mime_types {
            source.offer(mime.to_string());
        }

        source
    }

    /// create a new data source for a given seat with some user data
    pub fn create_data_control_source_with_data<D, U>(
        &self,
        qh: &QueueHandle<D>,
        data: U,
    ) -> ZwlrDataControlSourceV1
    where
        D: Dispatch<ZwlrDataControlSourceV1, U> + 'static,
        U: DataControlSourceDataExt + 'static,
    {
        self.manager.create_data_source(qh, data)
    }

    /// create a new data device for a given seat
    pub fn get_data_device<D>(&self, qh: &QueueHandle<D>, seat: &WlSeat) -> DataControlDevice
    where
        D: Dispatch<ZwlrDataControlDeviceV1, DataControlDeviceData> + 'static,
    {
        DataControlDevice {
            device: self.get_data_control_device_with_data(
                qh,
                seat,
                DataControlDeviceData::default(),
            ),
        }
    }

    /// create a new data device for a given seat with some user data
    pub fn get_data_control_device_with_data<D, U>(
        &self,
        qh: &QueueHandle<D>,
        seat: &WlSeat,
        data: U,
    ) -> ZwlrDataControlDeviceV1
    where
        D: Dispatch<ZwlrDataControlDeviceV1, U> + 'static,
        U: DataControlDeviceDataExt + 'static,
    {
        self.manager.get_data_device(seat, qh, data)
    }
}

impl ProvidesBoundGlobal<ZwlrDataControlManagerV1, 2> for DataControlDeviceManagerState {
    fn bound_global(&self) -> Result<ZwlrDataControlManagerV1, GlobalError> {
        Ok(self.manager.clone())
    }
}

impl<D> Dispatch<ZwlrDataControlManagerV1, GlobalData, D> for DataControlDeviceManagerState
where
    D: Dispatch<ZwlrDataControlManagerV1, GlobalData>,
{
    fn event(
        _state: &mut D,
        _proxy: &ZwlrDataControlManagerV1,
        _event: <ZwlrDataControlManagerV1 as Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}
