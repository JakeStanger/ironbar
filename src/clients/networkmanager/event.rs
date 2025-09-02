use crate::clients::networkmanager::dbus::{DeviceState, DeviceType};

#[derive(Debug, Clone)]
pub enum ClientToModuleEvent {
    DeviceChanged {
        interface: String,
        r#type: DeviceType,
        new_state: DeviceState,
    },
    DeviceRemoved {
        interface: String,
    },
}

#[derive(Debug, Clone)]
pub enum ModuleToClientEvent {
    NewController,
}
