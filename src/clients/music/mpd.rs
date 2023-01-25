use super::{MusicClient, Status, Track};
use crate::await_sync;
use crate::clients::music::{PlayerState, PlayerUpdate};
use color_eyre::Result;
use lazy_static::lazy_static;
use mpd_client::client::{Connection, ConnectionEvent, Subsystem};
use mpd_client::protocol::MpdProtocolError;
use mpd_client::responses::{PlayState, Song};
use mpd_client::tag::Tag;
use mpd_client::{commands, Client};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UnixStream};
use tokio::spawn;
use tokio::sync::broadcast::{channel, error::SendError, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error};

lazy_static! {
    static ref CONNECTIONS: Arc<Mutex<HashMap<String, Arc<MpdClient>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub struct MpdClient {
    client: Client,
    music_dir: PathBuf,
    tx: Sender<PlayerUpdate>,
    _rx: Receiver<PlayerUpdate>,
}

#[derive(Debug)]
pub enum MpdConnectionError {
    MaxRetries,
    ProtocolError(MpdProtocolError),
}

impl Display for MpdConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxRetries => write!(f, "Reached max retries"),
            Self::ProtocolError(e) => write!(f, "{e:?}"),
        }
    }
}

impl std::error::Error for MpdConnectionError {}

impl MpdClient {
    async fn new(host: &str, music_dir: PathBuf) -> Result<Self, MpdConnectionError> {
        debug!("Creating new MPD connection to {}", host);

        let (client, mut state_changes) =
            wait_for_connection(host, Duration::from_secs(5), None).await?;

        let (tx, rx) = channel(16);

        {
            let music_dir = music_dir.clone();
            let tx = tx.clone();
            let client = client.clone();

            spawn(async move {
                while let Some(change) = state_changes.next().await {
                    debug!("Received state change: {:?}", change);

                    if let ConnectionEvent::SubsystemChange(
                        Subsystem::Player | Subsystem::Queue | Subsystem::Mixer,
                    ) = change
                    {
                        Self::send_update(&client, &tx, &music_dir).await?;
                    }
                }

                Ok::<(), SendError<(Option<Track>, Status)>>(())
            });
        }

        Ok(Self {
            client,
            music_dir,
            tx,
            _rx: rx,
        })
    }

    async fn send_update(
        client: &Client,
        tx: &Sender<PlayerUpdate>,
        music_dir: &Path,
    ) -> Result<(), SendError<(Option<Track>, Status)>> {
        let current_song = client.command(commands::CurrentSong).await;
        let status = client.command(commands::Status).await;

        if let (Ok(current_song), Ok(status)) = (current_song, status) {
            let track = current_song.map(|s| Self::convert_song(&s.song, music_dir));
            let status = Status::from(status);

            tx.send((track, status))?;
        }

        Ok(())
    }

    fn convert_song(song: &Song, music_dir: &Path) -> Track {
        let (track, disc) = song.number();

        let cover_path = music_dir.join(
            song.file_path()
                .parent()
                .expect("Song path should not be root")
                .join("cover.jpg"),
        );

        Track {
            title: song.title().map(std::string::ToString::to_string),
            album: song.album().map(std::string::ToString::to_string),
            artist: Some(song.artists().join(", ")),
            date: try_get_first_tag(song, &Tag::Date).map(std::string::ToString::to_string),
            genre: try_get_first_tag(song, &Tag::Genre).map(std::string::ToString::to_string),
            disc: Some(disc),
            track: Some(track),
            cover_path: Some(cover_path),
        }
    }
}

macro_rules! async_command {
    ($client:expr, $command:expr) => {
        await_sync(async {
            $client
                .command($command)
                .await
                .unwrap_or_else(|err| error!("Failed to send command: {err:?}"))
        })
    };
}

impl MusicClient for MpdClient {
    fn play(&self) -> Result<()> {
        async_command!(self.client, commands::SetPause(false));
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        async_command!(self.client, commands::SetPause(true));
        Ok(())
    }

    fn next(&self) -> Result<()> {
        async_command!(self.client, commands::Next);
        Ok(())
    }

    fn prev(&self) -> Result<()> {
        async_command!(self.client, commands::Previous);
        Ok(())
    }

    fn set_volume_percent(&self, vol: u8) -> Result<()> {
        async_command!(self.client, commands::SetVolume(vol));
        Ok(())
    }

    fn subscribe_change(&self) -> Receiver<PlayerUpdate> {
        let rx = self.tx.subscribe();
        await_sync(async {
            Self::send_update(&self.client, &self.tx, &self.music_dir)
                .await
                .expect("Failed to send player update");
        });
        rx
    }
}

pub async fn get_client(
    host: &str,
    music_dir: PathBuf,
) -> Result<Arc<MpdClient>, MpdConnectionError> {
    let mut connections = CONNECTIONS.lock().await;
    match connections.get(host) {
        None => {
            let client = MpdClient::new(host, music_dir).await?;
            let client = Arc::new(client);
            connections.insert(host.to_string(), Arc::clone(&client));
            Ok(client)
        }
        Some(client) => Ok(Arc::clone(client)),
    }
}

async fn wait_for_connection(
    host: &str,
    interval: Duration,
    max_retries: Option<usize>,
) -> Result<Connection, MpdConnectionError> {
    let mut retries = 0;
    let max_retries = max_retries.unwrap_or(usize::MAX);

    loop {
        if retries == max_retries {
            break Err(MpdConnectionError::MaxRetries);
        }

        retries += 1;

        match try_get_mpd_conn(host).await {
            Ok(conn) => break Ok(conn),
            Err(err) => {
                if retries == max_retries {
                    break Err(MpdConnectionError::ProtocolError(err));
                }
            }
        }

        sleep(interval).await;
    }
}

/// Cycles through each MPD host and
/// returns the first one which connects,
/// or none if there are none
async fn try_get_mpd_conn(host: &str) -> Result<Connection, MpdProtocolError> {
    if is_unix_socket(host) {
        connect_unix(host).await
    } else {
        connect_tcp(host).await
    }
}

fn is_unix_socket(host: &str) -> bool {
    let path = PathBuf::from(host);
    path.exists()
        && path
            .metadata()
            .map_or(false, |metadata| metadata.file_type().is_socket())
}

async fn connect_unix(host: &str) -> Result<Connection, MpdProtocolError> {
    let connection = UnixStream::connect(host).await?;
    Client::connect(connection).await
}

async fn connect_tcp(host: &str) -> Result<Connection, MpdProtocolError> {
    let connection = TcpStream::connect(host).await?;
    Client::connect(connection).await
}

/// Attempts to read the first value for a tag
/// (since the MPD client returns a vector of tags, or None)
pub fn try_get_first_tag<'a>(song: &'a Song, tag: &'a Tag) -> Option<&'a str> {
    song.tags
        .get(tag)
        .and_then(|vec| vec.first().map(String::as_str))
}

impl From<mpd_client::responses::Status> for Status {
    fn from(status: mpd_client::responses::Status) -> Self {
        Self {
            state: PlayerState::from(status.state),
            volume_percent: status.volume,
            duration: status.duration,
            elapsed: status.elapsed,
            playlist_position: status.current_song.map_or(0, |(pos, _)| pos.0 as u32),
            playlist_length: status.playlist_length as u32,
        }
    }
}

impl From<PlayState> for PlayerState {
    fn from(value: PlayState) -> Self {
        match value {
            PlayState::Stopped => Self::Stopped,
            PlayState::Playing => Self::Playing,
            PlayState::Paused => Self::Paused,
        }
    }
}
