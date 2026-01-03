use crate::config::{CommonConfig, LayoutConfig, MarqueeMode, TruncateMode, default};
use dirs::{audio_dir, home_dir};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum PlayerType {
    #[cfg(feature = "music+mpd")]
    Mpd,
    #[cfg(feature = "music+mpris")]
    Mpris,
}

#[allow(clippy::derivable_impls)]
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
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct MusicModule {
    /// Type of player to connect to.
    #[serde(default)]
    #[cfg_attr(feature = "extras", schemars(extend("default" = "'mpris'")))]
    pub(crate) player_type: PlayerType,

    /// Format of current song info to display on the bar.
    ///
    /// The following tokens can be used and will be replaced
    /// with values from the currently playing track:
    ///
    /// | Token        | Description                          |
    /// |--------------|--------------------------------------|
    /// | `{title}`    | Title                                |
    /// | `{album}`    | Album name                           |
    /// | `{artist}`   | Artist name                          |
    /// | `{date}`     | Release date                         |
    /// | `{track}`    | Track number                         |
    /// | `{disc}`     | Disc number                          |
    /// | `{genre}`    | Genre                                |
    ///
    /// **Default**: `{title} / {artist}`
    pub(crate) format: String,

    /// Player state icons.
    ///
    /// See [icons](#icons).
    pub(crate) icons: Icons,

    /// Whether to show the play/pause status icon
    /// on the bar.
    ///
    /// **Default**: `true`
    pub(crate) show_status_icon: bool,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    pub(crate) icon_size: i32,

    /// Size to render the album art image at inside the popup, in pixels.
    ///
    /// **Default**: `128`
    pub(crate) cover_image_size: i32,

    // -- MPD --
    /// :::note
    /// MPD only
    /// :::
    ///
    /// TCP or Unix socket address of the MPD server.
    /// For TCP, this should include the port number.
    ///
    /// **Default**: `localhost:6600`
    pub(crate) host: String,

    /// :::note
    /// MPD only
    /// :::
    ///
    /// Path to root of the MPD server's music directory.
    /// This is required for displaying album art.
    ///
    /// **Default**: `$HOME/Music`
    pub(crate) music_dir: PathBuf,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate: Option<TruncateMode>,

    /// See [marquee options](module-level-options#marquee-mode).
    #[serde(default)]
    pub(crate) marquee: MarqueeMode,

    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate_popup_artist: Option<TruncateMode>,

    /// See [marquee options](module-level-options#marquee-mode).
    #[serde(default)]
    pub(crate) marquee_popup_artist: MarqueeMode,

    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate_popup_album: Option<TruncateMode>,

    /// See [marquee options](module-level-options#marquee-mode).
    #[serde(default)]
    pub(crate) marquee_popup_album: MarqueeMode,

    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate_popup_title: Option<TruncateMode>,

    /// See [marquee options](module-level-options#marquee-mode).
    #[serde(default)]
    pub(crate) marquee_popup_title: MarqueeMode,

    /// See [layout options](module-level-options#layout)
    #[serde(flatten)]
    pub(crate) layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for MusicModule {
    fn default() -> Self {
        Self {
            player_type: PlayerType::default(),
            format: "{title} / {artist}".to_string(),
            icons: Icons::default(),
            show_status_icon: true,
            icon_size: default::IconSize::Normal as i32,
            cover_image_size: 128,
            host: "localhost:6600".to_string(),
            music_dir: default_music_dir(),
            truncate: None,
            marquee: MarqueeMode::default(),
            truncate_popup_artist: None,
            marquee_popup_artist: MarqueeMode::default(),
            truncate_popup_album: None,
            marquee_popup_album: MarqueeMode::default(),
            truncate_popup_title: None,
            marquee_popup_title: MarqueeMode::default(),
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Icons {
    /// Icon to display when playing.
    ///
    /// **Default**: ``
    pub(crate) play: String,

    /// Icon to display when paused.
    ///
    /// **Default**: ``
    pub(crate) pause: String,

    /// Icon to display for previous button.
    ///
    /// **Default**: `󰒮`
    pub(crate) prev: String,

    /// Icon to display for next button.
    ///
    /// **Default**: `󰒭`
    pub(crate) next: String,

    /// Icon to display under volume slider.
    ///
    /// **Default**: `󰕾`
    pub(crate) volume: String,

    /// Icon to display nex to track title.
    ///
    /// **Default**: `󰎈`
    pub(crate) track: String,

    /// Icon to display nex to album name.
    ///
    /// **Default**: `󰀥`
    pub(crate) album: String,

    /// Icon to display nex to artist name.
    ///
    /// **Default**: `󰠃`
    pub(crate) artist: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            pause: "".to_string(),
            play: "".to_string(),
            prev: "󰒮".to_string(),
            next: "󰒭".to_string(),
            volume: "󰕾".to_string(),
            track: "󰎈".to_string(),
            album: "󰀥".to_string(),
            artist: "󰠃".to_string(),
        }
    }
}

fn default_music_dir() -> PathBuf {
    audio_dir().unwrap_or_else(|| home_dir().map(|dir| dir.join("Music")).unwrap_or_default())
}
