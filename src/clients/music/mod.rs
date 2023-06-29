use color_eyre::Result;
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
    /// Triggered when the client disconnects from the player.
    Disconnect,
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

#[derive(Clone, Copy, Debug)]
pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Copy, Debug)]
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

pub trait MusicClient {
    fn play(&self) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn next(&self) -> Result<()>;
    fn prev(&self) -> Result<()>;

    fn set_volume_percent(&self, vol: u8) -> Result<()>;
    fn seek(&self, duration: Duration) -> Result<()>;

    fn subscribe_change(&self) -> broadcast::Receiver<PlayerUpdate>;
}

pub enum ClientType<'a> {
    Mpd { host: &'a str, music_dir: PathBuf },
    Mpris,
}

pub async fn get_client(client_type: ClientType<'_>) -> Box<Arc<dyn MusicClient>> {
    match client_type {
        ClientType::Mpd { host, music_dir } => Box::new(
            mpd::get_client(host, music_dir)
                .await
                .expect("Failed to connect to MPD client"),
        ),
        ClientType::Mpris => Box::new(mpris::get_client()),
    }
}
