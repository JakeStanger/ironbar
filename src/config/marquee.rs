use serde::Deserialize;

/// Defines the behavior of marquee scrolling on hover.
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum MarqueeOnHover {
    /// Scrolling is always active, hover has no effect.
    #[default]
    None,
    /// Scrolling pauses when the widget is hovered.
    Pause,
    /// Scrolling only occurs when the widget is hovered.
    Play,
}

/// Some modules provide options for scrolling text (marquee effect).
/// This is controlled using a common `MarqueeMode` type,
/// which is defined below.
///
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct MarqueeMode {
    /// Whether to enable scrolling on long lines of text.
    /// This may not be supported by all modules.
    ///
    /// **Default**: `false`
    #[serde(default)]
    pub enable: bool,

    /// The maximum length of text (roughly, in characters) before it gets truncated and starts scrolling.
    ///
    /// **Default**: `null`
    #[serde(default)]
    pub max_length: Option<i32>,

    /// Scroll speed in pixels per frame.
    /// Higher values scroll faster.
    ///
    /// **Default**: `0.5`
    #[serde(default)]
    pub scroll_speed: Option<f64>,

    /// Duration in milliseconds to pause at each loop point.
    ///
    /// **Default**: `5000` (5 seconds)
    #[serde(default)]
    pub pause_duration: Option<u64>,

    /// Separator string to place between the repeated text.
    /// Can be any string, like " • " or " | ".
    ///
    /// **Default**: `"    "` (4 spaces)
    #[serde(default)]
    pub separator: Option<String>,

    /// Controls marquee behavior on hover.
    ///
    /// **Options**:
    /// - `"none"`: Always scroll (default)
    /// - `"pause"`: Pause scrolling on hover
    /// - `"play"`: Only scroll on hover
    ///
    /// **Default**: `"none"`
    #[serde(default)]
    pub on_hover: MarqueeOnHover,
}
