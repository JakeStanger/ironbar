use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use dirs::{audio_dir, home_dir};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Icons {
    /// Icon to display when playing.
    ///
    /// **Default**: ``
    #[serde(default = "default_icon_play")]
    pub(crate) play: String,

    /// Icon to display when paused.
    ///
    /// **Default**: ``
    #[serde(default = "default_icon_pause")]
    pub(crate) pause: String,

    /// Icon to display for previous button.
    ///
    /// **Default**: `󰒮`
    #[serde(default = "default_icon_prev")]
    pub(crate) prev: String,

    /// Icon to display for next button.
    ///
    /// **Default**: `󰒭`
    #[serde(default = "default_icon_next")]
    pub(crate) next: String,

    /// Icon to display under volume slider.
    ///
    /// **Default**: `󰕾`
    #[serde(default = "default_icon_volume")]
    pub(crate) volume: String,

    /// Icon to display nex to track title.
    ///
    /// **Default**: `󰎈`
    #[serde(default = "default_icon_track")]
    pub(crate) track: String,

    /// Icon to display nex to album name.
    ///
    /// **Default**: `󰀥`
    #[serde(default = "default_icon_album")]
    pub(crate) album: String,

    /// Icon to display nex to artist name.
    ///
    /// **Default**: `󰠃`
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PlayerType {
    #[cfg(feature = "music+mpd")]
    Mpd,
    #[cfg(feature = "music+mpris")]
    Mpris,
}

impl Default for PlayerType {
    fn default() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(feature = "music+mpris")] {
                Self::Mpris
            } else if #[cfg(feature = "music+mpd")] {
                Self::Mpd
            } else {
                compile_error!("No player type feature enabled")
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MusicModule {
    /// Type of player to connect to
    #[serde(default)]
    pub(crate) player_type: PlayerType,

    /// Format of current song info to display on the bar.
    ///
    /// Info on formatting tokens [below](#formatting-tokens).
    ///
    /// **Default**: `{title} / {artist}`
    #[serde(default = "default_format")]
    pub(crate) format: String,

    /// Player state icons.
    ///
    /// See [icons](#icons).
    #[serde(default)]
    pub(crate) icons: Icons,

    /// Whether to show the play/pause status icon
    /// on the bar.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    pub(crate) show_status_icon: bool,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    #[serde(default = "default_icon_size")]
    pub(crate) icon_size: i32,

    /// Size to render the album art image at inside the popup, in pixels.
    ///
    /// **Default**: `128`
    #[serde(default = "default_cover_image_size")]
    pub(crate) cover_image_size: i32,

    // -- MPD --
    /// *[MPD Only]*
    /// TCP or Unix socket address of the MPD server.
    /// For TCP, this should include the port number.
    ///
    /// **Default**: `localhost:6600`
    #[serde(default = "default_socket")]
    pub(crate) host: String,

    /// *[MPD Only]*
    /// Path to root of the MPD server's music directory.
    /// This is required for displaying album art.
    ///
    /// **Default**: `$HOME/Music`
    #[serde(default = "default_music_dir")]
    pub(crate) music_dir: PathBuf,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate: Option<TruncateMode>,

    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate_popup_artist: Option<TruncateMode>,

    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate_popup_album: Option<TruncateMode>,

    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate_popup_title: Option<TruncateMode>,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    pub(crate) layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
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
    String::from("󰒮")
}

fn default_icon_next() -> String {
    String::from("󰒭")
}

fn default_icon_volume() -> String {
    String::from("󰕾")
}

fn default_icon_track() -> String {
    String::from("󰎈")
}

fn default_icon_album() -> String {
    String::from("󰀥")
}

fn default_icon_artist() -> String {
    String::from("󰠃")
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
