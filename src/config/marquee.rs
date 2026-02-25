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
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct MarqueeMode {
    /// Whether to enable scrolling on long lines of text.
    /// This may not be supported by all modules.
    ///
    /// **Default**: `false`
    pub enable: bool,

    /// The maximum length of text (roughly, in characters) before it gets truncated and starts scrolling.
    ///
    /// **Default**: `null`
    pub max_length: Option<i32>,

    /// Scroll speed in pixels per frame.
    /// Higher values scroll faster.
    ///
    /// **Default**: `0.5`
    pub scroll_speed: f64,

    /// Duration in milliseconds to pause at each loop point.
    ///
    /// **Default**: `5000` (5 seconds)
    pub pause_duration: u64,

    /// String displayed between the end and beginning of text as it loops.
    ///
    /// **Default**: `"    "` (4 spaces)
    pub separator: String,

    /// Controls marquee behavior on hover.
    ///
    /// **Options**:
    /// - `"none"`: Always scroll (default)
    /// - `"pause"`: Pause scrolling on hover
    /// - `"play"`: Only scroll on hover
    ///
    /// **Default**: `"none"`
    pub on_hover: MarqueeOnHover,
}

impl Default for MarqueeMode {
    fn default() -> Self {
        Self {
            enable: false,
            max_length: None,
            scroll_speed: 0.5,
            pause_duration: 5000,
            separator: "    ".to_string(),
            on_hover: MarqueeOnHover::default(),
        }
    }
}
