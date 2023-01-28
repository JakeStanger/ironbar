use color_eyre::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

pub mod mpd;
pub mod mpris;

#[derive(Clone, Debug)]
pub enum PlayerUpdate {
    Update(Box<Option<Track>>, Status),
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
    pub cover_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub state: PlayerState,
    pub volume_percent: u8,
    pub duration: Option<Duration>,
    pub elapsed: Option<Duration>,
    pub playlist_position: u32,
    pub playlist_length: u32,
}

pub trait MusicClient {
    fn play(&self) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn next(&self) -> Result<()>;
    fn prev(&self) -> Result<()>;

    fn set_volume_percent(&self, vol: u8) -> Result<()>;

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
