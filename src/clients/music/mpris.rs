use super::{MusicClient, PlayerUpdate, Status, Track};
use crate::clients::music::PlayerState;
use crate::error::ERR_MUTEX_LOCK;
use color_eyre::Result;
use lazy_static::lazy_static;
use mpris::{DBusError, Event, Metadata, PlaybackStatus, Player, PlayerFinder};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::task::spawn_blocking;
use tracing::{debug, error, trace};

lazy_static! {
    static ref CLIENT: Arc<Client> = Arc::new(Client::new());
}

pub struct Client {
    current_player: Arc<Mutex<Option<String>>>,
    tx: Sender<PlayerUpdate>,
    _rx: Receiver<PlayerUpdate>,
}

impl Client {
    fn new() -> Self {
        let (tx, rx) = channel(32);

        let current_player = Arc::new(Mutex::new(None));

        {
            let players_list = Arc::new(Mutex::new(HashSet::new()));
            let current_player = current_player.clone();
            let tx = tx.clone();

            spawn_blocking(move || {
                let player_finder = PlayerFinder::new().expect("Failed to connect to D-Bus");

                // D-Bus gives no event for new players,
                // so we have to keep polling the player list
                loop {
                    let players = player_finder
                        .find_all()
                        .expect("Failed to connect to D-Bus");

                    let mut players_list_val = players_list.lock().expect(ERR_MUTEX_LOCK);
                    for player in players {
                        let identity = player.identity();

                        if !players_list_val.contains(identity) {
                            debug!("Adding MPRIS player '{identity}'");
                            players_list_val.insert(identity.to_string());

                            let status = player
                                .get_playback_status()
                                .expect("Failed to connect to D-Bus");

                            {
                                let mut current_player =
                                    current_player.lock().expect(ERR_MUTEX_LOCK);

                                if status == PlaybackStatus::Playing || current_player.is_none() {
                                    debug!("Setting active player to '{identity}'");

                                    current_player.replace(identity.to_string());
                                    if let Err(err) = Self::send_update(&player, &tx) {
                                        error!("{err:?}");
                                    }
                                }
                            }

                            Self::listen_player_events(
                                identity.to_string(),
                                players_list.clone(),
                                current_player.clone(),
                                tx.clone(),
                            );
                        }
                    }

                    // wait 1 second before re-checking players
                    sleep(Duration::from_secs(1));
                }
            });
        }

        Self {
            current_player,
            tx,
            _rx: rx,
        }
    }

    fn listen_player_events(
        player_id: String,
        players: Arc<Mutex<HashSet<String>>>,
        current_player: Arc<Mutex<Option<String>>>,
        tx: Sender<PlayerUpdate>,
    ) {
        spawn_blocking(move || {
            let player_finder = PlayerFinder::new()?;

            if let Ok(player) = player_finder.find_by_name(&player_id) {
                let identity = player.identity();

                for event in player.events()? {
                    trace!("Received player event from '{identity}': {event:?}");
                    match event {
                        Ok(Event::PlayerShutDown) => {
                            current_player.lock().expect(ERR_MUTEX_LOCK).take();
                            players.lock().expect(ERR_MUTEX_LOCK).remove(identity);
                            break;
                        }
                        Ok(Event::Playing) => {
                            current_player
                                .lock()
                                .expect(ERR_MUTEX_LOCK)
                                .replace(identity.to_string());

                            if let Err(err) = Self::send_update(&player, &tx) {
                                error!("{err:?}");
                            }
                        }
                        Ok(_) => {
                            let current_player = current_player.lock().expect(ERR_MUTEX_LOCK);
                            let current_player = current_player.as_ref();
                            if let Some(current_player) = current_player {
                                if current_player == identity {
                                    if let Err(err) = Self::send_update(&player, &tx) {
                                        error!("{err:?}");
                                    }
                                }
                            }
                        }
                        Err(err) => error!("{err:?}"),
                    }
                }
            }

            Ok::<(), DBusError>(())
        });
    }

    fn send_update(player: &Player, tx: &Sender<PlayerUpdate>) -> Result<()> {
        debug!("Sending update using '{}'", player.identity());

        let metadata = player.get_metadata()?;
        let playback_status = player
            .get_playback_status()
            .unwrap_or(PlaybackStatus::Stopped);

        let track_list = player.get_track_list();

        let volume_percent = player
            .get_volume()
            .map(|vol| (vol * 100.0) as u8)
            .unwrap_or(0);

        let status = Status {
            playlist_position: 0,
            playlist_length: track_list.map(|list| list.len() as u32).unwrap_or(1),
            state: PlayerState::from(playback_status),
            elapsed: player.get_position().ok(),
            duration: metadata.length(),
            volume_percent,
        };

        let track = Track::from(metadata);

        let player_update: PlayerUpdate = (Some(track), status);

        tx.send(player_update)
            .expect("Failed to send player update");

        Ok(())
    }

    fn get_player(&self) -> Option<Player> {
        let player_name = self.current_player.lock().expect(ERR_MUTEX_LOCK);
        let player_name = player_name.as_ref();

        player_name.and_then(|player_name| {
            let player_finder = PlayerFinder::new().expect("Failed to connect to D-Bus");
            player_finder.find_by_name(player_name).ok()
        })
    }
}

macro_rules! command {
    ($self:ident, $func:ident) => {
        if let Some(player) = Self::get_player($self) {
            player.$func()?;
        } else {
            error!("Could not find player");
        }
    };
}

impl MusicClient for Client {
    fn play(&self) -> Result<()> {
        command!(self, play);
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        command!(self, pause);
        Ok(())
    }

    fn next(&self) -> Result<()> {
        command!(self, next);
        Ok(())
    }

    fn prev(&self) -> Result<()> {
        command!(self, previous);
        Ok(())
    }

    fn set_volume_percent(&self, vol: u8) -> Result<()> {
        if let Some(player) = Self::get_player(self) {
            player.set_volume(vol as f64 / 100.0)?;
        } else {
            error!("Could not find player");
        }
        Ok(())
    }

    fn subscribe_change(&self) -> Receiver<PlayerUpdate> {
        debug!("Creating new subscription");
        let rx = self.tx.subscribe();

        if let Some(player) = self.get_player() {
            if let Err(err) = Self::send_update(&player, &self.tx) {
                error!("{err:?}");
            }
        }

        rx
    }
}

pub fn get_client() -> Arc<Client> {
    CLIENT.clone()
}

impl From<Metadata> for Track {
    fn from(value: Metadata) -> Self {
        const KEY_DATE: &str = "xesam:contentCreated";
        const KEY_GENRE: &str = "xesam:genre";

        Self {
            title: value.title().map(std::string::ToString::to_string),
            album: value.album_name().map(std::string::ToString::to_string),
            artist: value.artists().map(|artists| artists.join(", ")),
            date: value
                .get(KEY_DATE)
                .and_then(mpris::MetadataValue::as_string)
                .map(std::string::ToString::to_string),
            disc: value.disc_number().map(|disc| disc as u64),
            genre: value
                .get(KEY_GENRE)
                .and_then(mpris::MetadataValue::as_str_array)
                .and_then(|arr| arr.first().map(|val| (*val).to_string())),
            track: value.track_number().map(|track| track as u64),
            cover_path: value
                .art_url()
                .map(|path| path.replace("file://", ""))
                .map(PathBuf::from),
        }
    }
}

impl From<PlaybackStatus> for PlayerState {
    fn from(value: PlaybackStatus) -> Self {
        match value {
            PlaybackStatus::Playing => Self::Playing,
            PlaybackStatus::Paused => Self::Paused,
            PlaybackStatus::Stopped => Self::Stopped,
        }
    }
}
