use super::{
    MusicClient, PlayerState, PlayerUpdate, ProgressTick, Status, Track, TICK_INTERVAL_MS,
};
use crate::{await_sync, send, spawn, Ironbar};
use color_eyre::Report;
use color_eyre::Result;
use mpd_client::client::{ConnectionEvent, Subsystem};
use mpd_client::commands::{self, SeekMode};
use mpd_client::responses::{PlayState, Song};
use mpd_client::tag::Tag;
use mpd_utils::mpd_client::commands::Command;
use mpd_utils::mpd_client::responses::TypedResponseError;
use mpd_utils::{mpd_client, PersistentClient};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tracing::debug;

macro_rules! command {
    ($self:ident, $command:expr) => {
        await_sync(async move { $self.client.command($command).await.map_err(Report::new) })
    };
}

#[derive(Debug)]
pub struct Client {
    client: Arc<PersistentClient>,
    music_dir: PathBuf,
    tx: broadcast::Sender<PlayerUpdate>,
    _rx: broadcast::Receiver<PlayerUpdate>,
}

impl Client {
    pub fn new(host: String, music_dir: PathBuf) -> Self {
        let client = Arc::new(PersistentClient::new(host, Duration::from_secs(5)));
        let mut client_rx = client.subscribe();

        let (tx, rx) = broadcast::channel(32);

        let _guard = Ironbar::runtime().enter();
        client.init();

        {
            let tx = tx.clone();
            let client = client.clone();
            let music_dir = music_dir.clone();

            spawn(async move {
                Self::send_update(&client, &tx, &music_dir)
                    .await
                    .expect("Failed to send update");

                while let Ok(change) = client_rx.recv().await {
                    debug!("Received state change: {change:?}");
                    if let ConnectionEvent::SubsystemChange(
                        Subsystem::Player | Subsystem::Queue | Subsystem::Mixer,
                    ) = *change
                    {
                        Self::send_update(&client, &tx, &music_dir)
                            .await
                            .expect("Failed to send update");
                    }
                }
            });
        }

        {
            let tx = tx.clone();
            let client = client.clone();

            spawn(async move {
                loop {
                    Self::send_tick_update(&client, &tx).await;
                    sleep(Duration::from_millis(TICK_INTERVAL_MS)).await;
                }
            });
        }

        Self {
            client,
            tx,
            music_dir,
            _rx: rx,
        }
    }

    async fn send_update(
        client: &PersistentClient,
        tx: &broadcast::Sender<PlayerUpdate>,
        music_dir: &Path,
    ) -> Result<(), broadcast::error::SendError<PlayerUpdate>> {
        let current_song = client.command(commands::CurrentSong).await;
        let status = client.command(commands::Status).await;

        if let (Ok(current_song), Ok(status)) = (current_song, status) {
            let track = current_song
                .clone()
                .map(|s| convert_song(&s.song, music_dir));
            let status = Status::from(status);

            let update_info = PlayerUpdate::Update(Box::new(track), status);
            send!(tx, update_info);
        }

        Ok(())
    }

    async fn send_tick_update(client: &PersistentClient, tx: &broadcast::Sender<PlayerUpdate>) {
        let status = client.command(commands::Status).await;

        if let Ok(status) = status {
            if status.state == PlayState::Playing {
                let update = PlayerUpdate::ProgressTick(ProgressTick {
                    duration: status.duration,
                    elapsed: status.elapsed,
                });

                send!(tx, update);
            }
        }
    }
}

fn convert_song(song: &Song, music_dir: &Path) -> Track {
    let (track, disc) = song.number();

    let cover_path = music_dir
        .join(
            song.file_path()
                .parent()
                .expect("Song Path should not be the root")
                .join("cover.jpg"),
        )
        .into_os_string()
        .into_string()
        .ok();

    Track {
        title: song.title().map(ToString::to_string),
        album: song.album().map(ToString::to_string),
        artist: Some(song.artists().join(", ")),
        date: try_get_first_tag(song, &Tag::Date).map(ToString::to_string),
        genre: try_get_first_tag(song, &Tag::Genre).map(ToString::to_string),
        disc: Some(disc),
        track: Some(track),
        uri: Some(song.file_path().display().to_string()),
        cover_path,
    }
}
pub async fn get_picture(
    client: &PersistentClient,
    uri: &str,
) -> Result<Vec<u8>, TypedResponseError> {
    let mut offset = 0;

    let mut slice = client
        .command(ReadPicture {
            uri: uri.to_string(),
            offset,
        })
        .await
        .map_err(Report::new)
        .map_err(|e| {
            tracing::error!("{e:#?}");
            TypedResponseError::missing("cover art")
        })?;
    let total_length = slice.0;
    let mut buffer = Vec::with_capacity(total_length as usize);
    offset += slice.1.len();
    buffer
        .write_all(slice.1.as_slice())
        .expect("Writing to in memory buffer");
    while offset < total_length as usize {
        slice = client
            .command(ReadPicture {
                uri: uri.to_string(),
                offset,
            })
            .await
            .map_err(Report::new)
            .map_err(|e| {
                tracing::error!("{e:#?}");
                TypedResponseError::missing("cover art")
            })?;
        offset += slice.1.len();
        buffer
            .write_all(slice.1.as_slice())
            .expect("Writing to an in memory buffer");
    }
    Write::flush(&mut buffer).unwrap();
    Ok(buffer)
}

impl MusicClient for Client {
    fn play(&self) -> Result<()> {
        command!(self, commands::SetPause(false))
    }

    fn pause(&self) -> Result<()> {
        command!(self, commands::SetPause(true))
    }

    fn next(&self) -> Result<()> {
        command!(self, commands::Next)
    }

    fn prev(&self) -> Result<()> {
        command!(self, commands::Previous)
    }

    fn set_volume_percent(&self, vol: u8) -> Result<()> {
        command!(self, commands::SetVolume(vol))
    }

    fn seek(&self, duration: Duration) -> Result<()> {
        command!(self, commands::Seek(SeekMode::Absolute(duration)))
    }

    fn subscribe_change(&self) -> broadcast::Receiver<PlayerUpdate> {
        let rx = self.tx.subscribe();
        await_sync(async move {
            Self::send_update(&self.client, &self.tx, &self.music_dir)
                .await
                .expect("to be able to send update");
        });
        rx
    }

    fn send_album_art(&self, uri: String) -> Result<()> {
        let cover_image = await_sync(get_picture(self.client.as_ref(), uri.as_str())).ok();
        let update_image = PlayerUpdate::UpdateImage(cover_image);
        send!(self.tx, update_image);
        Ok(())
    }
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
            volume_percent: Some(status.volume),
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

pub struct ReadPicture {
    pub uri: String,
    pub offset: usize,
}

impl Command for ReadPicture {
    type Response = (u32, Vec<u8>);

    fn command(&self) -> mpd_client::protocol::Command {
        mpd_client::protocol::Command::new("readpicture")
            .argument(self.uri.clone())
            .argument(self.offset)
    }

    fn response(
        self,
        frame: mpd_client::protocol::response::Frame,
    ) -> Result<Self::Response, mpd_client::responses::TypedResponseError> {
        if frame.is_empty() || !frame.has_binary() {
            Err(TypedResponseError::missing("id3v2 thumbnail"))
        } else {
            Ok((
                frame
                    .find("size")
                    .expect("Having a size field")
                    .parse()
                    .expect("Getting a unsigned int for the size field"),
                frame.binary().map(|b| b.to_vec()).unwrap(),
            ))
        }
    }
}
