mod client;
mod popup;

use self::popup::Popup;
use crate::modules::mpd::client::{get_connection, get_duration, get_elapsed};
use crate::modules::mpd::popup::{MpdPopup, PopupEvent};
use crate::modules::{Module, ModuleInfo};
use color_eyre::Result;
use dirs::{audio_dir, home_dir};
use glib::Continue;
use gtk::prelude::*;
use gtk::{Button, Orientation};
use mpd_client::commands::responses::{PlayState, Song, Status};
use mpd_client::{commands, Tag};
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::error;

#[derive(Debug, Deserialize, Clone)]
pub struct MpdModule {
    #[serde(default = "default_socket")]
    host: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default = "default_icon_play")]
    icon_play: Option<String>,
    #[serde(default = "default_icon_pause")]
    icon_pause: Option<String>,

    #[serde(default = "default_music_dir")]
    music_dir: PathBuf,
}

fn default_socket() -> String {
    String::from("localhost:6600")
}

fn default_format() -> String {
    String::from("{icon}  {title} / {artist}")
}

#[allow(clippy::unnecessary_wraps)]
fn default_icon_play() -> Option<String> {
    Some(String::from(""))
}

#[allow(clippy::unnecessary_wraps)]
fn default_icon_pause() -> Option<String> {
    Some(String::from(""))
}

fn default_music_dir() -> PathBuf {
    audio_dir().unwrap_or_else(|| home_dir().map(|dir| dir.join("Music")).unwrap_or_default())
}

/// Attempts to read the first value for a tag
/// (since the MPD client returns a vector of tags, or None)
pub fn try_get_first_tag(vec: Option<&Vec<String>>) -> Option<&str> {
    match vec {
        Some(vec) => vec.first().map(String::as_str),
        None => None,
    }
}

/// Formats a duration given in seconds
/// in hh:mm format
fn format_time(time: u64) -> String {
    let minutes = (time / 60) % 60;
    let seconds = time % 60;

    format!("{:0>2}:{:0>2}", minutes, seconds)
}

/// Extracts the formatting tokens from a formatting string
fn get_tokens(re: &Regex, format_string: &str) -> Vec<String> {
    re.captures_iter(format_string)
        .map(|caps| caps[1].to_string())
        .collect::<Vec<_>>()
}

enum Event {
    Open,
    Update(Box<Option<(Song, Status, String)>>),
}

impl Module<Button> for MpdModule {
    fn into_widget(self, info: &ModuleInfo) -> Result<Button> {
        let re = Regex::new(r"\{([\w-]+)}")?;
        let tokens = get_tokens(&re, self.format.as_str());

        let button = Button::new();

        let (ui_tx, mut ui_rx) = mpsc::channel(32);

        let popup = Popup::new(
            "popup-mpd",
            info.app,
            info.monitor,
            Orientation::Horizontal,
            info.bar_position,
        );
        let mpd_popup = MpdPopup::new(popup, ui_tx);

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let click_tx = tx.clone();

        let music_dir = self.music_dir.clone();

        button.connect_clicked(move |_| {
            click_tx
                .send(Event::Open)
                .expect("Failed to send popup open event");
        });

        let host = self.host.clone();
        let host2 = self.host.clone();
        spawn(async move {
            let client = get_connection(&host)
                .await
                .expect("Unexpected error when trying to connect to MPD server");

            loop {
                let current_song = client.command(commands::CurrentSong).await;
                let status = client.command(commands::Status).await;

                if let (Ok(Some(song)), Ok(status)) = (current_song, status) {
                    let string = self
                        .replace_tokens(self.format.as_str(), &tokens, &song.song, &status)
                        .await;

                    tx.send(Event::Update(Box::new(Some((song.song, status, string)))))
                        .expect("Failed to send update event");
                } else {
                    tx.send(Event::Update(Box::new(None)))
                        .expect("Failed to send update event");
                }

                sleep(Duration::from_secs(1)).await;
            }
        });

        spawn(async move {
            let client = get_connection(&host2)
                .await
                .expect("Unexpected error when trying to connect to MPD server");

            while let Some(event) = ui_rx.recv().await {
                let res = match event {
                    PopupEvent::Previous => client.command(commands::Previous).await,
                    PopupEvent::Toggle => match client.command(commands::Status).await {
                        Ok(status) => match status.state {
                            PlayState::Playing => client.command(commands::SetPause(true)).await,
                            PlayState::Paused => client.command(commands::SetPause(false)).await,
                            PlayState::Stopped => Ok(()),
                        },
                        Err(err) => Err(err),
                    },
                    PopupEvent::Next => client.command(commands::Next).await,
                };

                if let Err(err) = res {
                    error!("Failed to send command to MPD server: {:?}", err);
                }
            }
        });

        {
            let button = button.clone();

            rx.attach(None, move |event| {
                match event {
                    Event::Open => {
                        mpd_popup.popup.show(&button);
                    }
                    Event::Update(mut msg) => {
                        if let Some((song, status, string)) = msg.take() {
                            mpd_popup.update(&song, &status, music_dir.as_path());

                            button.set_label(&string);
                            button.show();
                        } else {
                            button.hide();
                        }
                    }
                }

                Continue(true)
            });
        };

        Ok(button)
    }
}

impl MpdModule {
    /// Replaces each of the formatting tokens in the formatting string
    /// with actual data pulled from MPD
    async fn replace_tokens(
        &self,
        format_string: &str,
        tokens: &Vec<String>,
        song: &Song,
        status: &Status,
    ) -> String {
        let mut compiled_string = format_string.to_string();
        for token in tokens {
            let value = self.get_token_value(song, status, token).await;
            compiled_string =
                compiled_string.replace(format!("{{{}}}", token).as_str(), value.as_str());
        }
        compiled_string
    }

    /// Converts a string format token value
    /// into its respective MPD value.
    pub async fn get_token_value(&self, song: &Song, status: &Status, token: &str) -> String {
        let s = match token {
            "icon" => {
                let icon = match status.state {
                    PlayState::Stopped => None,
                    PlayState::Playing => self.icon_play.as_ref(),
                    PlayState::Paused => self.icon_pause.as_ref(),
                };
                icon.map(String::as_str)
            }
            "title" => song.title(),
            "album" => try_get_first_tag(song.tags.get(&Tag::Album)),
            "artist" => try_get_first_tag(song.tags.get(&Tag::Artist)),
            "date" => try_get_first_tag(song.tags.get(&Tag::Date)),
            "disc" => try_get_first_tag(song.tags.get(&Tag::Disc)),
            "genre" => try_get_first_tag(song.tags.get(&Tag::Genre)),
            "track" => try_get_first_tag(song.tags.get(&Tag::Track)),
            "duration" => return get_duration(status).map(format_time).unwrap_or_default(),

            "elapsed" => return get_elapsed(status).map(format_time).unwrap_or_default(),
            _ => Some(token),
        };
        s.unwrap_or_default().to_string()
    }
}
