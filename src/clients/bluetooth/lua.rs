use super::{BluetoothDevice, BluetoothDeviceStatus, BluetoothState};
use crate::clients::lua::into_lua_debug;

impl mlua::UserData for BluetoothState {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("kind", |_, this| Ok(this.to_string()));
        fields.add_field_method_get("enabled", |_, this| Ok(this.is_enabled()));
        fields.add_field_method_get("devices", |lua, this| {
            let devices = match this {
                Self::Enabled { devices } => devices.as_slice(),
                _ => &[],
            };
            lua.create_sequence_from(devices.iter().cloned())
        });
    }
}

into_lua_debug!(BluetoothDeviceStatus);

#[mlua::userdata_impl]
impl BluetoothDevice {
    #[lua(getter, name = "address", infallible)]
    fn lua_address(&self) -> String {
        self.address.to_string()
    }
}
