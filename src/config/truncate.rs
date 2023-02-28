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

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(untagged)]
pub enum TruncateMode {
    Auto(EllipsizeMode),
    Length {
        mode: EllipsizeMode,
        length: Option<i32>,
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
