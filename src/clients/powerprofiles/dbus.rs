//! Power Profiles D-BUS interface binding.
//! upstream documentation: <https://upower.pages.freedesktop.org/power-profiles-daemon/gdbus-org.freedesktop.UPower.PowerProfiles.html>

use std::collections::HashMap;

use zbus::{self, Str, proxy};
pub(super) const OBJECT_PATH: &str = "/org/freedesktop/UPower/PowerProfiles";
pub(super) const SERVICE: &str = "org.freedesktop.UPower.PowerProfiles";

// Properties being watched
pub(super) const ACTIVE_PROFILE: &str = "ActiveProfile";
pub(super) const PERFORMANCE_DEGRADED: &str = "PerformanceDegraded";

#[proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles"
)]
pub trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<Box<str>>;

    #[zbus(property)]
    fn set_active_profile(&self, value: Str<'_>) -> zbus::Result<()>;

    #[zbus(property)]
    fn profiles(&self) -> zbus::Result<Vec<HashMap<Box<str>, Box<str>>>>;

    #[zbus(property)]
    fn performance_degraded(&self) -> zbus::Result<Box<str>>;

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
