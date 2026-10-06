use super::Event;

impl mlua::UserData for Event {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("kind", |_, this| Ok(this.to_string()));
        fields.add_field_method_get("value", |_, this| {
            Ok(match this {
                Self::ActiveProfile(profile) => Some(profile.to_string()),
                Self::PerformanceDegraded(reason) => reason.as_deref().map(str::to_string),
            })
        });
    }
}
