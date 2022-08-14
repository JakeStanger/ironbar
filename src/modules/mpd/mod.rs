mod client;
mod popup;

use self::popup::Popup;
use crate::modules::mpd::client::{get_connection, get_duration, get_elapsed};
use crate::modules::mpd::popup::{MpdPopup, PopupEvent};
use crate::modules::{Module, ModuleInfo};
use crate::popup::PopupAlignment;
use dirs::home_dir;
use glib::Continue;
use gtk::prelude::*;
use gtk::{Button, Orientation};
use mpd_client::commands::responses::{PlayState, Song, Status};
use mpd_client::{commands, Tag};
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::time::sleep;

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

fn default_icon_play() -> Option<String> {
    Some(String::from(""))
}

fn default_icon_pause() -> Option<String> {
    Some(String::from(""))
}

fn default_music_dir() -> PathBuf {
    home_dir().unwrap().join("Music")
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
    Open(f64),
    Update(Box<Option<(Song, Status, String)>>),
}

impl Module<Button> for MpdModule {
    fn into_widget(self, info: &ModuleInfo) -> Button {
        let re = Regex::new(r"\{([\w-]+)}").unwrap();
        let tokens = get_tokens(&re, self.format.as_str());

        let button = Button::new();

        let (ui_tx, mut ui_rx) = mpsc::channel(32);

        let popup = Popup::new("popup-mpd", info.app, Orientation::Horizontal);
        let mpd_popup = MpdPopup::new(popup, ui_tx);

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let click_tx = tx.clone();

        let music_dir = self.music_dir.clone();

        button.connect_clicked(move |button| {
            let button_w = button.allocation().width();

            let (button_x, _) = button
                .translate_coordinates(&button.toplevel().unwrap(), 0, 0)
                .unwrap();

            click_tx
                .send(Event::Open(f64::from(button_x + button_w)))
                .unwrap();
        });

        let host = self.host.clone();
        let host2 = self.host.clone();
        spawn(async move {
            let (client, _) = get_connection(&host).await.unwrap(); // TODO: Handle connecting properly

            loop {
                let current_song = client.command(commands::CurrentSong).await;
                let status = client.command(commands::Status).await;

                if let (Ok(Some(song)), Ok(status)) = (current_song, status) {
                    let string = self
                        .replace_tokens(self.format.as_str(), &tokens, &song.song, &status)
                        .await;

                    tx.send(Event::Update(Box::new(Some((song.song, status, string)))))
                        .unwrap();
                } else {
                    tx.send(Event::Update(Box::new(None))).unwrap();
                }

                sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        spawn(async move {
            let (client, _) = get_connection(&host2).await.unwrap(); // TODO: Handle connecting properly

            while let Some(event) = ui_rx.recv().await {
                match event {
                    PopupEvent::Previous => client.command(commands::Previous).await,
                    PopupEvent::Toggle => {
                        let status = client.command(commands::Status).await.unwrap();
                        match status.state {
                            PlayState::Playing => client.command(commands::SetPause(true)).await,
                            PlayState::Paused => client.command(commands::SetPause(false)).await,
                            PlayState::Stopped => Ok(())
                        }
                    }
                    PopupEvent::Next => client.command(commands::Next).await
                }.unwrap();
            }
        });

        {
            let button = button.clone();

            rx.attach(None, move |event| {
                match event {
                    Event::Open(pos) => {
                        mpd_popup.popup.show();
                        mpd_popup.popup.set_pos(pos, PopupAlignment::Right);
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

        button
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
                icon.map(|i| i.as_str())
            }
            "title" => song.title(),
            "album" => try_get_first_tag(song.tags.get(&Tag::Album)),
            "artist" => try_get_first_tag(song.tags.get(&Tag::Artist)),
            "date" => try_get_first_tag(song.tags.get(&Tag::Date)),
            "disc" => try_get_first_tag(song.tags.get(&Tag::Disc)),
            "genre" => try_get_first_tag(song.tags.get(&Tag::Genre)),
            "track" => try_get_first_tag(song.tags.get(&Tag::Track)),
            "duration" => return format_time(get_duration(status)),
            "elapsed" => return format_time(get_elapsed(status)),
            _ => return token.to_string(),
        };
        s.unwrap_or_default().to_string()
    }
}
