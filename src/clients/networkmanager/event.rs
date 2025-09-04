use crate::clients::networkmanager::dbus::{DeviceState, DeviceType};

#[derive(Debug, Clone)]
pub enum ClientToModuleEvent {
    DeviceChanged {
        number: u32,
        r#type: DeviceType,
        new_state: DeviceState,
    },
    DeviceRemoved {
        number: u32,
    },
}

#[derive(Debug, Clone)]
pub enum ModuleToClientEvent {
    NewController,
}
