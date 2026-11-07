use super::{Event, Key};
use crate::clients::lua::into_lua_debug;

into_lua_debug!(Key);

impl mlua::UserData for Event {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("kind", |_, this| Ok(this.to_string()));
        fields.add_field_method_get("key", |_, this| {
            Ok(match this {
                Self::Key(ev) => Some(ev.key),
                Self::Device => None,
            })
        });
        fields.add_field_method_get("state", |_, this| {
            Ok(match this {
                Self::Key(ev) => Some(ev.state),
                Self::Device => None,
            })
        });
    }
}
