use crate::config::{
    CommonConfig, LayoutConfig, MarqueeMode, ModuleOrientation, Profiles, TruncateMode,
};
use crate::profiles;
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct VolumeProfile {
    pub(super) icons: Icons,
}

impl VolumeProfile {
    fn for_volume_icon(icon: &str) -> Self {
        Self {
            icons: Icons {
                volume: icon.to_string(),
                ..Icons::default()
            },
        }
    }
}

pub(super) fn default_profiles() -> Profiles<f64, VolumeProfile> {
    profiles!(
        "low":33.0 => VolumeProfile::for_volume_icon("󰕿"),
        "medium":66.66 => VolumeProfile::for_volume_icon("󰖀")
    )
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct VolumeModule {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{icon} {percentage}%`
    pub(super) format: String,

    /// Maximum value to allow volume sliders to reach.
    /// Pulse supports values > 100 but this may result in distortion.
    ///
    /// **Default**: `100`
    pub(super) max_volume: f64,

    /// The orientation of the sink slider
    ///
    /// **Default**: vertical
    pub(super) sink_slider_orientation: ModuleOrientation,

    /// See [profiles](profiles).
    #[serde(flatten)]
    pub(super) profiles: Profiles<f64, VolumeProfile>,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate: Option<TruncateMode>,

    /// See [marquee options](module-level-options#marquee-mode).
    #[serde(default)]
    pub(crate) marquee: MarqueeMode,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    pub(super) layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for VolumeModule {
    fn default() -> Self {
        Self {
            format: "{icon} {percentage}%".to_string(),
            max_volume: 100.0,
            sink_slider_orientation: ModuleOrientation::Vertical,
            profiles: Profiles::default(),
            truncate: None,
            marquee: MarqueeMode::default(),
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Icons {
    /// Icon to show to represent each volume level.
    ///
    ///  **Default**: `󰕾`
    pub(super) volume: String,

    /// Icon to show for muted outputs.
    ///
    /// **Default**: `󰝟`
    pub(super) muted: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            volume: "󰕾".to_string(),
            muted: "󰝟".to_string(),
        }
    }
}
