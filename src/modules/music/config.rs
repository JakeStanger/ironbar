use crate::config::{CommonConfig, TruncateMode};
use dirs::{audio_dir, home_dir};
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

    /// Icon to display for previous button.
    #[serde(default = "default_icon_prev")]
    pub(crate) prev: String,

    /// Icon to display for next button.
    #[serde(default = "default_icon_next")]
    pub(crate) next: String,

    /// Icon to display under volume slider
    #[serde(default = "default_icon_volume")]
    pub(crate) volume: String,

    /// Icon to display nex to track title
    #[serde(default = "default_icon_track")]
    pub(crate) track: String,

    /// Icon to display nex to album name
    #[serde(default = "default_icon_album")]
    pub(crate) album: String,

    /// Icon to display nex to artist name
    #[serde(default = "default_icon_artist")]
    pub(crate) artist: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            pause: default_icon_pause(),
            play: default_icon_play(),
            prev: default_icon_prev(),
            next: default_icon_next(),
            volume: default_icon_volume(),
            track: default_icon_track(),
            album: default_icon_album(),
            artist: default_icon_artist(),
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

    // -- MPD --
    /// TCP or Unix socket address.
    #[serde(default = "default_socket")]
    pub(crate) host: String,
    /// Path to root of music directory.
    #[serde(default = "default_music_dir")]
    pub(crate) music_dir: PathBuf,

    #[serde(default = "crate::config::default_true")]
    pub(crate) show_status_icon: bool,

    #[serde(default = "default_icon_size")]
    pub(crate) icon_size: i32,

    #[serde(default = "default_cover_image_size")]
    pub(crate) cover_image_size: i32,

    // -- Common --
    pub(crate) truncate: Option<TruncateMode>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_socket() -> String {
    String::from("localhost:6600")
}

fn default_format() -> String {
    String::from("{title} / {artist}")
}

fn default_icon_play() -> String {
    String::from("")
}

fn default_icon_pause() -> String {
    String::from("")
}

fn default_icon_prev() -> String {
    String::from("\u{f9ad}")
}

fn default_icon_next() -> String {
    String::from("\u{f9ac}")
}

fn default_icon_volume() -> String {
    String::from("墳")
}

fn default_icon_track() -> String {
    String::from("\u{f886}")
}

fn default_icon_album() -> String {
    String::from("\u{f524}")
}

fn default_icon_artist() -> String {
    String::from("\u{fd01}")
}

fn default_music_dir() -> PathBuf {
    audio_dir().unwrap_or_else(|| home_dir().map(|dir| dir.join("Music")).unwrap_or_default())
}

const fn default_icon_size() -> i32 {
    24
}

const fn default_cover_image_size() -> i32 {
    128
}
