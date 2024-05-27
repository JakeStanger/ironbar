use gtk::pango::EllipsizeMode as GtkEllipsizeMode;
use gtk::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum EllipsizeMode {
    Start,
    Middle,
    End,
}

impl From<EllipsizeMode> for GtkEllipsizeMode {
    fn from(value: EllipsizeMode) -> Self {
        match value {
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
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(untagged)]
pub enum TruncateMode {
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
    /// **Default**: `null`
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

impl TruncateMode {
    const fn mode(&self) -> EllipsizeMode {
        match self {
            Self::Length { mode, .. } | Self::Auto(mode) => *mode,
        }
    }

    const fn length(&self) -> Option<i32> {
        match self {
            Self::Auto(_) => None,
            Self::Length { length, .. } => *length,
        }
    }

    const fn max_length(&self) -> Option<i32> {
        match self {
            Self::Auto(_) => None,
            Self::Length { max_length, .. } => *max_length,
        }
    }

    pub fn truncate_label(&self, label: &gtk::Label) {
        label.set_ellipsize(self.mode().into());

        if let Some(length) = self.length() {
            label.set_width_chars(length);
        }

        if let Some(length) = self.max_length() {
            label.set_max_width_chars(length);
        }
    }
}
