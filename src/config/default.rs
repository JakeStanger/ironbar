/// Command for launching external applications.
///
/// `gtk-launch {app_name}`
pub fn launch_command() -> String {
    String::from("gtk-launch {app_name}")
}

/// Image icon sizes.
#[repr(i32)]
pub enum IconSize {
    /// 32
    Normal = 32,
    /// 24
    Small = 24,
    /// 16
    Tiny = 16,
}
