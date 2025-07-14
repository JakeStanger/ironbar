use color_eyre::Result;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[cfg(feature = "music+mpd")]
pub mod mpd;
#[cfg(feature = "music+mpris")]
pub mod mpris;

pub const TICK_INTERVAL_MS: u64 = 200;

#[derive(Clone, Debug)]
pub enum PlayerUpdate {
    /// Triggered when the track or player state notably changes,
    /// such as a new track playing, the player being paused, or a volume change.
    Update(Box<Option<Track>>, Status),
    /// Triggered at regular intervals while a track is playing.
    /// Used to keep track of the progress through the current track.
    ProgressTick(ProgressTick),
}

#[derive(Clone, Debug)]
pub struct Track {
    pub title: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub date: Option<String>,
    pub disc: Option<u64>,
    pub genre: Option<String>,
    pub track: Option<u64>,
    pub cover_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum PlayerState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Status {
    pub state: PlayerState,
    pub volume_percent: Option<u8>,
    pub playlist_position: u32,
    pub playlist_length: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ProgressTick {
    pub duration: Option<Duration>,
    pub elapsed: Option<Duration>,
}

pub trait MusicClient: Debug + Send + Sync {
    fn play(&self) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn next(&self) -> Result<()>;
    fn prev(&self) -> Result<()>;

    fn set_volume_percent(&self, vol: u8) -> Result<()>;
    fn seek(&self, duration: Duration) -> Result<()>;

    fn subscribe_change(&self) -> broadcast::Receiver<PlayerUpdate>;
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ClientType {
    #[cfg(feature = "music+mpd")]
    Mpd { host: String, music_dir: PathBuf },
    #[cfg(feature = "music+mpris")]
    Mpris,
}

pub fn create_client(client_type: ClientType) -> Arc<dyn MusicClient> {
    match client_type {
        #[cfg(feature = "music+mpd")]
        ClientType::Mpd { host, music_dir } => Arc::new(mpd::Client::new(host, music_dir)),
        #[cfg(feature = "music+mpris")]
        ClientType::Mpris => Arc::new(mpris::Client::new()),
    }
}
