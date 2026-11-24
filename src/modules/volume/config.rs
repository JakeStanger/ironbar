use std::collections::HashMap;

use crate::config::{
    CommonConfig, LayoutConfig, MarqueeMode, ModuleOrientation, Profiles, TruncateMode,
};
use crate::profiles;
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
#[serde(default)]
pub struct VolumeProfile {
    pub(super) icons: Icons,
}

impl VolumeProfile {
    fn for_volume_icon(sink_icon: &str) -> Self {
        Self {
            icons: Icons {
                volume: sink_icon.to_string(),
                ..Icons::default()
            },
        }
    }
}

pub(super) fn default_profiles() -> Profiles<f64, VolumeProfile> {
    profiles!(
        "low":33.33 => VolumeProfile::for_volume_icon("󰕿"),
        "medium":66.66 => VolumeProfile::for_volume_icon("󰖀")
    )
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
#[serde(default)]
pub struct VolumeModule {
    /// The format string to use for the widget button label.
    ///
    /// Available tokens:
    ///
    /// | Token          | Description                               |
    /// |----------------|-------------------------------------------|
    /// | `{percentage}` | The active device volume percentage.      |
    /// | `{icon}`       | The icon representing the current volume. |
    /// | `{name}`       | The active device name.                   |
    pub(super) format: String,

    /// The format string to use for the widget button label when muted.
    ///
    /// Available tokens:
    ///
    /// | Token          | Description                               |
    /// |----------------|-------------------------------------------|
    /// | `{percentage}` | The active device volume percentage.      |
    /// | `{icon}`       | The icon representing the current volume. |
    /// | `{name}`       | The active device name.                   |
    pub(super) mute_format: String,

    /// Maximum value to allow volume sliders to reach.
    /// Pulse supports values > 100 but this may result in distortion.
    pub(super) max_volume: f64,

    /// The orientation of elements in popup
    pub(super) popup_orientation: ModuleOrientation,

    /// The orientation of the sink slider
    pub(super) sink_slider_orientation: ModuleOrientation,

    /// The orientation of the source slider
    pub(super) source_slider_orientation: ModuleOrientation,

    /// Show sinks (speakers) on the bar and in the popup.
    pub(super) show_sinks: bool,

    /// Show sources (microphones) on the bar and in the popup.
    pub(super) show_sources: bool,

    /// Show pulseaudio sink monitors for mic outputs
    pub(super) show_monitors: bool,

    #[serde(flatten)]
    pub(super) profiles: Profiles<f64, VolumeProfile>,

    /// Allows to overwrite device description with a custom one.
    /// Use `{name}` to reuse the device name in the new one.
    /// Add `-muted` for when the sink or source is muted.
    pub(super) name_map: HashMap<Box<str>, String>,

    // -- Common --
    /// Controls the truncation options of text displayed on the bar.
    pub(crate) truncate: Option<TruncateMode>,

    /// Controls the truncation options of device and application labels displayed in the popup.
    pub(crate) truncate_popup: Option<TruncateMode>,

    /// Controls the marquee mode for application labels displayed in the popup.
    #[serde(default)]
    pub(crate) marquee: MarqueeMode,

    #[serde(default, flatten)]
    pub(super) layout: LayoutConfig,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,

    /// Control by amount in percent a scroll moves the volume levels.
    /// Use Negative values to inverse scroll direction.
    /// By default the direction uses natural scroll.
    /// To disable scrolling use `null`, 0 or `nan`.
    pub(crate) scroll_step: Option<f64>,
}

impl Default for VolumeModule {
    fn default() -> Self {
        Self {
            format: "{icon} {percentage}%".to_string(),
            mute_format: "{icon} {percentage}%".to_string(),
            max_volume: 100.0,
            popup_orientation: ModuleOrientation::Horizontal,
            sink_slider_orientation: ModuleOrientation::Horizontal,
            source_slider_orientation: ModuleOrientation::Horizontal,
            show_sinks: true,
            show_sources: true,
            show_monitors: false,
            profiles: Profiles::default(),
            name_map: HashMap::new(),
            truncate: None,
            truncate_popup: None,
            marquee: MarqueeMode::default(),
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
            scroll_step: Some(1.0),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
#[serde(default)]
pub struct Icons {
    /// Icon to show to represent each volume level.
    pub(super) volume: String,

    /// Icon to show for muted outputs.
    pub(super) muted: String,

    /// Icon to show to represent each microphone volume level.
    pub(super) mic_volume: String,

    /// Icon to show for muted inputs.
    pub(super) mic_muted: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            volume: "󰕾".to_string(),
            muted: "󰝟".to_string(),
            mic_volume: "".to_string(),
            mic_muted: "".to_string(),
        }
    }
}
