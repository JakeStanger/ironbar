use crate::clients::networkmanager::dbus::{DeviceState, DeviceType};

#[derive(Debug, Clone)]
pub enum Event {
    DeviceAdded {
        interface: String,
        r#type: DeviceType,
    },
    DeviceStateChanged {
        interface: String,
        r#type: DeviceType,
        state: DeviceState,
    },
}
