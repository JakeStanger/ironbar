use gtk::pango::EllipsizeMode as GtkEllipsizeMode;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum EllipsizeMode {
    None,
    Start,
    Middle,
    #[default]
    End,
}

impl From<EllipsizeMode> for GtkEllipsizeMode {
    fn from(value: EllipsizeMode) -> Self {
        match value {
            EllipsizeMode::None => Self::None,
            EllipsizeMode::Start => Self::Start,
            EllipsizeMode::Middle => Self::Middle,
            EllipsizeMode::End => Self::End,
        }
    }
}

/// Some modules provide options for truncating text.
/// This is controlled using a common `TruncateMode` type,
/// which is defined below.
///
/// The option can be configured in one of two modes.
///
/// **Default**: `Auto (end)`
///
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(untagged)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum TruncateMode {
    /// Do not truncate content.
    ///
    /// Setting this option may cause excessively long content to overflow other widgets,
    /// shifting them off-screen.
    ///
    /// # Example
    ///
    /// ```corn
    /// { truncate = "off" }
    Off,

    /// Auto mode lets GTK decide when to ellipsize.
    ///
    /// To use this mode, set the truncate option to a string
    /// declaring the location to truncate text from and place the ellipsis.
    ///
    /// # Example
    ///
    /// ```corn
    /// { truncate = "start" }
    /// ```
    ///
    /// **Valid options**: `start`, `middle`, `end`
    /// <br>
    /// **Default**: `end`
    Auto(EllipsizeMode),

    /// Length mode defines a fixed point at which to ellipsize.
    ///
    /// Generally you will want to set only one of `length` or `max_length`,
    /// but you can set both if required.
    ///
    /// # Example
    ///
    /// ```corn
    /// {
    ///     truncate.mode = "start"
    ///     truncate.length = 50
    ///     truncate.max_length = 70
    /// }
    /// ```
    Length {
        /// The location to truncate text from and place the ellipsis.
        /// **Valid options**: `start`, `middle`, `end`
        /// <br>
        /// **Default**: `null`
        mode: EllipsizeMode,

        /// The fixed width (in characters) of the widget.
        ///
        /// The widget will be expanded to this width
        /// if it would have otherwise been smaller.
        ///
        /// Leave unset to let GTK automatically handle.
        ///
        /// **Default**: `null`
        length: Option<i32>,

        /// The maximum number of characters to show
        /// before truncating.
        ///
        /// Leave unset to let GTK automatically handle.
        ///
        /// **Default**: `null`
        max_length: Option<i32>,
    },
}

impl Default for TruncateMode {
    fn default() -> Self {
        Self::Auto(EllipsizeMode::default())
    }
}

impl TruncateMode {
    pub const fn length(&self) -> Option<i32> {
        match self {
            Self::Auto(_) | Self::Off => None,
            Self::Length { length, .. } => *length,
        }
    }

    pub const fn max_length(&self) -> Option<i32> {
        match self {
            Self::Auto(_) | Self::Off => None,
            Self::Length { max_length, .. } => *max_length,
        }
    }
}

impl From<TruncateMode> for GtkEllipsizeMode {
    fn from(value: TruncateMode) -> Self {
        let mode = match value {
            TruncateMode::Off => EllipsizeMode::None,
            TruncateMode::Length { mode, .. } | TruncateMode::Auto(mode) => mode,
        };
        mode.into()
    }
}
