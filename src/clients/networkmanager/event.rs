use crate::clients::networkmanager::dbus::{DeviceState, DeviceType};

#[derive(Debug, Clone)]
pub enum ClientToModuleEvent {
    DeviceStateChanged {
        interface: String,
        r#type: DeviceType,
        state: DeviceState,
    },
    DeviceRemoved {
        interface: String,
    },
}

#[derive(Debug, Clone)]
pub enum ModuleToClientEvent {
    NewController,
}
