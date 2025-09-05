use serde::Deserialize;

/// Some modules provide options for scrolling text (marquee effect).
/// This is controlled using a common `MarqueeMode` type,
/// which is defined below.
///
#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

    /// Whether to pause scrolling on hover.
    ///
    /// **Default**: `false`
    #[serde(default)]
    pub pause_on_hover: bool,

    /// Whether to invert the pause on hover behavior.
    /// When true, scrolling will only occur on hover.
    /// This takes priority over `pause_on_hover`.
    ///
    /// **Default**: `false`
    #[serde(default)]
    pub pause_on_hover_invert: bool,
}
