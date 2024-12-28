use super::{MusicClient, PlayerState, PlayerUpdate, Status, Track, TICK_INTERVAL_MS};
use crate::channels::SyncSenderExt;
use crate::clients::music::ProgressTick;
use crate::{arc_mut, lock, spawn_blocking};
use color_eyre::Result;
use mpris::{DBusError, Event, Metadata, PlaybackStatus, Player, PlayerFinder};
use std::cmp;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, trace};

#[derive(Debug)]
pub struct Client {
    current_player: Arc<Mutex<Option<String>>>,
    tx: broadcast::Sender<PlayerUpdate>,
    _rx: broadcast::Receiver<PlayerUpdate>,
}

const NO_ACTIVE_PLAYER: &str = "com.github.altdesktop.playerctld.NoActivePlayer";
const NO_REPLY: &str = "org.freedesktop.DBus.Error.NoReply";
const NO_SERVICE: &str = "org.freedesktop.DBus.Error.ServiceUnknown";
const NO_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";

impl Client {
    pub(crate) fn new() -> Self {
        let (tx, rx) = broadcast::channel(32);

        let current_player = arc_mut!(None);

        {
            let players_list = arc_mut!(HashSet::new());
            let current_player = current_player.clone();
            let tx = tx.clone();

            spawn_blocking(move || {
                let player_finder = PlayerFinder::new().expect("Failed to connect to D-Bus");

                // D-Bus gives no event for new players,
                // so we have to keep polling the player list
                loop {
                    // mpris-rs does not filter NoActivePlayer errors, so we have to do it ourselves
                    let players = player_finder.find_all().unwrap_or_else(|e| match e {
                        mpris::FindingError::DBusError(DBusError::TransportError(
                            transport_error,
                        )) if transport_error.name() == Some(NO_ACTIVE_PLAYER)
                            || transport_error.name() == Some(NO_REPLY) =>
                        {
                            vec![]
                        }
                        _ => {
                            error!("D-Bus error getting MPRIS players: {e:?}");
                            vec![]
                        }
                    });

                    // Acquire the lock of current_player before players to avoid deadlock.
                    // There are places where we lock on current_player and players, but we always lock on current_player first.
                    // This is because we almost never need to lock on players without locking on current_player.
                    {
                        let mut current_player_lock = lock!(current_player);

                        let mut players_list_val = lock!(players_list);
                        for player in players {
                            let identity = player.identity();

                            if current_player_lock.is_none() {
                                debug!("Setting active player to '{identity}'");
                                current_player_lock.replace(identity.to_string());

                                if let Err(err) = Self::send_update(&player, &tx) {
                                    error!("{err:?}");
                                }
                            }
                            if !players_list_val.contains(identity) {
                                debug!("Adding MPRIS player '{identity}'");
                                players_list_val.insert(identity.to_string());

                                Self::listen_player_events(
                                    identity.to_string(),
                                    players_list.clone(),
                                    current_player.clone(),
                                    tx.clone(),
                                );
                            }
                        }
                    }
                    // wait 1 second before re-checking players
                    sleep(Duration::from_secs(1));
                }
            });
        }

        {
            let current_player = current_player.clone();
            let tx = tx.clone();

            spawn_blocking(move || {
                let player_finder = PlayerFinder::new().expect("to get new player finder");

                loop {
                    Self::send_tick_update(&player_finder, &current_player, &tx);
                    sleep(Duration::from_millis(TICK_INTERVAL_MS));
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
        tx: broadcast::Sender<PlayerUpdate>,
    ) {
        spawn_blocking(move || {
            let player_finder = PlayerFinder::new()?;

            if let Ok(player) = player_finder.find_by_name(&player_id) {
                let identity = player.identity();
                let handle_shutdown = |current_player_lock_option: Option<
                    std::sync::MutexGuard<'_, Option<String>>,
                >| {
                    debug!("Player '{identity}' shutting down");
                    // Lock of player before players (see new() to make sure order is consistent)
                    if let Some(mut guard) = current_player_lock_option {
                        guard.take();
                    } else {
                        lock!(current_player).take();
                    }
                    let mut players_locked = lock!(players);
                    players_locked.remove(identity);
                    if players_locked.is_empty() {
                        tx.send_expect(PlayerUpdate::Update(Box::new(None), Status::default()));
                    }
                };

                for event in player.events()? {
                    trace!("Received player event from '{identity}': {event:?}");
                    match event {
                        Ok(Event::PlayerShutDown) => {
                            handle_shutdown(None);
                            break;
                        }
                        Err(mpris::EventError::DBusError(DBusError::TransportError(
                            transport_error,
                        ))) if transport_error.name() == Some(NO_ACTIVE_PLAYER)
                            || transport_error.name() == Some(NO_REPLY)
                            || transport_error.name() == Some(NO_METHOD)
                            || transport_error.name() == Some(NO_SERVICE) =>
                        {
                            handle_shutdown(None);
                            break;
                        }
                        Ok(_) => {
                            let mut current_player_lock = lock!(current_player);
                            if matches!(event, Ok(Event::Playing)) {
                                current_player_lock.replace(identity.to_string());
                            }
                            if let Some(current_identity) = current_player_lock.as_ref() {
                                if current_identity == identity {
                                    if let Err(err) = Self::send_update(&player, &tx) {
                                        if let Some(DBusError::TransportError(transport_error)) =
                                            err.downcast_ref::<DBusError>()
                                        {
                                            if transport_error.name() == Some(NO_SERVICE) {
                                                handle_shutdown(Some(current_player_lock));
                                                break;
                                            }
                                        }
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

    fn send_update(player: &Player, tx: &broadcast::Sender<PlayerUpdate>) -> Result<()> {
        debug!("Sending update using '{}'", player.identity());

        let metadata = player.get_metadata()?;
        let playback_status = player
            .get_playback_status()
            .unwrap_or(PlaybackStatus::Stopped);

        let track_list = player.get_track_list();

        let volume_percent = player.get_volume().map(|vol| (vol * 100.0) as u8).ok();

        let status = Status {
            // MRPIS doesn't seem to provide playlist info reliably,
            // so we can just assume next/prev will work by bodging the numbers
            playlist_position: 1,
            playlist_length: track_list.map(|list| list.len() as u32).unwrap_or(u32::MAX),
            state: PlayerState::from(playback_status),
            volume_percent,
        };

        let track = Track::from(metadata);

        let player_update = PlayerUpdate::Update(Box::new(Some(track)), status);
        tx.send_expect(player_update);

        Ok(())
    }

    fn get_player(&self) -> Option<Player> {
        let player_name = lock!(self.current_player);
        let player_name = player_name.as_ref();

        player_name.and_then(|player_name| {
            let player_finder = PlayerFinder::new().expect("Failed to connect to D-Bus");
            player_finder.find_by_name(player_name).ok()
        })
    }

    fn send_tick_update(
        player_finder: &PlayerFinder,
        current_player: &Mutex<Option<String>>,
        tx: &broadcast::Sender<PlayerUpdate>,
    ) {
        if let Some(player) = lock!(current_player)
            .as_ref()
            .and_then(|name| player_finder.find_by_name(name).ok())
        {
            if let Ok(metadata) = player.get_metadata() {
                let update = PlayerUpdate::ProgressTick(ProgressTick {
                    elapsed: player.get_position().ok(),
                    duration: metadata.length(),
                });

                tx.send_expect(update);
            }
        }
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
            player.set_volume(f64::from(vol) / 100.0)?;
        } else {
            error!("Could not find player");
        }
        Ok(())
    }

    fn seek(&self, duration: Duration) -> Result<()> {
        if let Some(player) = Self::get_player(self) {
            let pos = player.get_position().unwrap_or_default();

            let duration = duration.as_micros() as i64;
            let position = pos.as_micros() as i64;

            let seek = cmp::max(duration, 0) - position;

            player.seek(seek)?;
        } else {
            error!("Could not find player");
        }
        Ok(())
    }

    fn subscribe_change(&self) -> broadcast::Receiver<PlayerUpdate> {
        debug!("Creating new subscription");
        let rx = self.tx.subscribe();

        if let Some(player) = self.get_player() {
            if let Err(err) = Self::send_update(&player, &self.tx) {
                error!("{err:?}");
            }
        } else {
            let status = Status {
                playlist_position: 0,
                playlist_length: 0,
                state: PlayerState::Stopped,
                volume_percent: None,
            };

            self.tx
                .send_expect(PlayerUpdate::Update(Box::new(None), status));
        }

        rx
    }
}

impl From<Metadata> for Track {
    fn from(value: Metadata) -> Self {
        const KEY_DATE: &str = "xesam:contentCreated";
        const KEY_GENRE: &str = "xesam:genre";

        Self {
            title: value
                .title()
                .map(ToString::to_string)
                .and_then(replace_empty_none),
            album: value
                .album_name()
                .map(ToString::to_string)
                .and_then(replace_empty_none),
            artist: value
                .artists()
                .map(|artists| artists.join(", "))
                .and_then(replace_empty_none),
            date: value
                .get(KEY_DATE)
                .and_then(mpris::MetadataValue::as_string)
                .map(ToString::to_string),
            disc: value.disc_number().map(|disc| disc as u64),
            genre: value
                .get(KEY_GENRE)
                .and_then(mpris::MetadataValue::as_str_array)
                .and_then(|arr| arr.first().map(|val| (*val).to_string())),
            track: value.track_number().map(|track| track as u64),
            cover_path: value.art_url().map(ToString::to_string),
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

fn replace_empty_none(string: String) -> Option<String> {
    if string.is_empty() {
        None
    } else {
        Some(string)
    }
}
