//! Power Profiles D-BUS interface binding.
//! upstream documentation: <https://upower.pages.freedesktop.org/power-profiles-daemon/gdbus-org.freedesktop.UPower.PowerProfiles.html>

use std::collections::HashMap;

use zbus::{Result, proxy, zvariant::Str};

#[proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles"
)]
pub trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> Result<Str<'_>>;

    #[zbus(property)]
    fn set_active_profile(&self, value: Str<'_>) -> Result<()>;

    #[zbus(property)]
    fn profiles(&self) -> Result<Vec<HashMap<Str<'_>, Str<'_>>>>;

    #[zbus(property)]
    fn performance_degraded(&self) -> Result<Str<'_>>;

    // #[zbus(property)]
    // fn actions(&self) -> Result<Array<'_>>;

    // #[zbus(property)]
    // fn actions_info(&self) -> Result<Dict<'_, '_>>;

    // #[zbus(property)]
    // fn active_profile_holds(&self) -> Result<Dict<'_, '_>>;

    // #[zbus(property)]
    // fn version(&self) -> Result<Str<'_>>;

    // #[zbus(property)]
    // fn battery_aware(&self) -> Result<bool>;
}
