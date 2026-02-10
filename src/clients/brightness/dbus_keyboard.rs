use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.UPower.KbdBacklight",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/KbdBacklight"
)]
pub trait KbdBacklight {
    /// GetMaxBrightness method
    fn get_max_brightness(&self) -> zbus::Result<i32>;

    /// GetBrightness method
    fn get_brightness(&self) -> zbus::Result<i32>;

    /// SetBrightness method
    fn set_brightness(&self, value: i32) -> zbus::Result<()>;

    /// BrightnessChanged signal
    #[zbus(signal)]
    fn brightness_changed(&self, value: i32) -> zbus::Result<()>;

    /// BrightnessChangedWithSource signal
    #[zbus(signal)]
    fn brightness_changed_with_source(&self, value: i32, source: &str) -> zbus::Result<()>;
}
