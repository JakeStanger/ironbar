use crate::config::CommonConfig;
use dirs::{audio_dir, home_dir};
use gtk::pango::EllipsizeMode as GtkEllipsizeMode;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Icons {
    /// Icon to display when playing.
    #[serde(default = "default_icon_play")]
    pub(crate) play: String,
    /// Icon to display when paused.
    #[serde(default = "default_icon_pause")]
    pub(crate) pause: String,
    /// Icon to display under volume slider
    #[serde(default = "default_icon_volume")]
    pub(crate) volume: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            pause: default_icon_pause(),
            play: default_icon_play(),
            volume: default_icon_volume(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum PlayerType {
    Mpd,
    Mpris,
}

impl Default for PlayerType {
    fn default() -> Self {
        Self::Mpris
    }
}

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
    MaxLength {
        mode: EllipsizeMode,
        length: Option<i32>,
    },
}

impl TruncateMode {
    pub(crate) const fn mode(&self) -> EllipsizeMode {
        match self {
            Self::MaxLength { mode, .. } | Self::Auto(mode) => *mode,
        }
    }

    pub(crate) const fn length(&self) -> Option<i32> {
        match self {
            Self::Auto(_) => None,
            Self::MaxLength { length, .. } => *length,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MusicModule {
    /// Type of player to connect to
    #[serde(default)]
    pub(crate) player_type: PlayerType,

    /// Format of current song info to display on the bar.
    #[serde(default = "default_format")]
    pub(crate) format: String,

    /// Player state icons
    #[serde(default)]
    pub(crate) icons: Icons,

    pub(crate) truncate: Option<TruncateMode>,

    // -- MPD --
    /// TCP or Unix socket address.
    #[serde(default = "default_socket")]
    pub(crate) host: String,
    /// Path to root of music directory.
    #[serde(default = "default_music_dir")]
    pub(crate) music_dir: PathBuf,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_socket() -> String {
    String::from("localhost:6600")
}

fn default_format() -> String {
    String::from("{icon}  {title} / {artist}")
}

fn default_icon_play() -> String {
    String::from("")
}

fn default_icon_pause() -> String {
    String::from("")
}

fn default_icon_volume() -> String {
    String::from("墳")
}

fn default_music_dir() -> PathBuf {
    audio_dir().unwrap_or_else(|| home_dir().map(|dir| dir.join("Music")).unwrap_or_default())
}
