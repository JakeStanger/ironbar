use serde::Deserialize;

use crate::{
    clients::brightness::{Key, default_kbd_ressource_name, default_resource_name},
    config::{CommonConfig, LayoutConfig, Profiles, TruncateMode},
    profiles,
};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrightnessDataSource {
    /// Uses D-Bus for keyboard devices.
    /// Use `busctl` to find the keyboard you want under interface `org.freedesktop.UPower`.
    /// You can check for the path `/org/freedesktop/UPower/KbdBacklight/<device>`.
    ///
    /// Note: this only works for keyboards, not for screen brightness.
    ///
    /// **Default**: `None`
    Keyboard { path: Option<Box<str>> },
    /// Uses the login1 D-Bus and fs for reading.
    /// This works for keyboard and screen brightness setting.
    /// It uses the filesystem for reading the data and D-Bus for adjusting.
    ///
    /// Using this method allows setting the brightness without video group permissions.
    Systemd {
        /// The subsystem to read the data from, e.g. `backlight` or `leds`.
        /// Subsystem refers to the directory within `/sys/class/`.
        ///
        /// **Default**: `backlight`
        subsystem: Box<str>,

        /// The name of the resource, see `/sys/class/<subsystem>` for available resources.
        /// If empty, Ironbar will try to resolve it via a hardcoded list of common resources
        ///
        /// **Default**: `None`
        name: Option<Box<str>>,
    },
}

const PATH_RESOLVED: &str = "path to be resolved";
const NAME_RESOLVED: &str = "name to be resolved";

impl BrightnessDataSource {
    /// Resolves empty field from [BrightnessDataSource].
    ///
    /// # Error
    ///
    /// When no default namespace could be resolved.
    pub(super) fn resolve(&mut self) -> Result<(), ()> {
        match self {
            BrightnessDataSource::Keyboard { path } => {
                if path.is_some() {
                    return Ok(());
                }

                *path = default_kbd_ressource_name();

                if path.is_none() {
                    return Err(());
                };
            }
            BrightnessDataSource::Systemd { subsystem, name } => {
                if name.is_some() {
                    return Ok(());
                }

                *name = default_resource_name(subsystem);

                if name.is_none() {
                    return Err(());
                }
            }
        }

        Ok(())
    }

    /// Get the [Key] for the [BrightnessDataSource] in use.
    pub(super) fn key(&self) -> Key {
        match self {
            BrightnessDataSource::Keyboard { path } => Key::Kbd {
                path: path.as_ref().expect(PATH_RESOLVED).clone(),
            },
            BrightnessDataSource::Systemd { subsystem, name } => Key::SysFs {
                subsystem: subsystem.clone(),
                device: name.as_ref().expect(NAME_RESOLVED).clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrightnessScaleControl {
    /// Attempts to select the scale for your device based on what your driver reports.
    /// The selected scale tried to best match perceived brightness and percentage values.
    #[default]
    Automatic,
    /// Treat brightness data from your device as linear. This is the legacy behavior.
    Linear,
    /// Use a Gamma 2.2 curve to interpret brightness values.
    Gamma2_2,
    /// Use a Gamma 2.4 curve to interpret brightness values. see [wiki](https://en.wikipedia.org/wiki/SRGB#Transfer_function_(%22gamma%22))
    Gamma2_4,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BrightnessProfile {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{percentage}%`
    pub(super) format: String,

    /// The icon to show on the bar widget button.
    /// Supports [image](images) icons.
    ///
    /// **Default**: null
    pub(super) icon_label: Option<String>,
}

impl Default for BrightnessProfile {
    fn default() -> Self {
        Self {
            format: "{percentage}%".to_string(),
            icon_label: None,
        }
    }
}

impl BrightnessProfile {
    fn with_icon(icon: &str) -> Self {
        Self {
            format: "{percentage}%".to_string(),
            icon_label: Some(icon.to_string()),
        }
    }
}

pub(super) fn default_profiles() -> Profiles<f64, BrightnessProfile> {
    profiles!(
        "level0":5.0 => BrightnessProfile::with_icon(""),
        "level10":15.0 => BrightnessProfile::with_icon(""),
        "level20":25.0 => BrightnessProfile::with_icon(""),
        "level30":35.0 => BrightnessProfile::with_icon(""),
        "level40":45.0 => BrightnessProfile::with_icon(""),
        "level50":55.0 => BrightnessProfile::with_icon(""),
        "level60":65.0 => BrightnessProfile::with_icon(""),
        "level70":75.0 => BrightnessProfile::with_icon(""),
        "level80":85.0 => BrightnessProfile::with_icon(""),
        "level90":95.0 => BrightnessProfile::with_icon(""),
        "level100":100.0 => BrightnessProfile::with_icon("")
    )
}

impl Default for BrightnessDataSource {
    fn default() -> Self {
        BrightnessDataSource::Systemd {
            subsystem: Box::from("backlight"),
            name: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BrightnessModule {
    /// See [profiles](profiles).
    #[serde(flatten)]
    pub(super) profiles: Profiles<f64, BrightnessProfile>,

    /// Where to get the brightness data from
    ///
    /// See [BrightnessDataSource].
    pub(super) mode: BrightnessDataSource,

    /// The method Ironbar should use for translating slider percentages to/from perceived brightness.
    ///
    /// See [BrightnessScaleControl].
    pub(super) scale: BrightnessScaleControl,

    /// A multiplier from to control the speed of smooth scrolling on trackpad.
    /// Choose a negative number to swap scrolling direction.
    ///
    /// **Default**: `1.0`
    pub(super) smooth_scroll_speed: f32,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(super) truncate: Option<TruncateMode>,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    pub(super) layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for BrightnessModule {
    fn default() -> Self {
        Self {
            profiles: Profiles::default(),
            mode: BrightnessDataSource::default(),
            scale: BrightnessScaleControl::default(),
            smooth_scroll_speed: 1.0,
            truncate: None,
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}
