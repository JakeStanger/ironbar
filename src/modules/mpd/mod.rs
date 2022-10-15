mod client;

use crate::modules::mpd::client::MpdConnectionError;
use crate::modules::mpd::client::{get_client, get_duration, get_elapsed};
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::Popup;
use color_eyre::Result;
use dirs::{audio_dir, home_dir};
use glib::Continue;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{Button, Image, Label, Orientation};
use mpd_client::commands;
use mpd_client::responses::{PlayState, Song, Status};
use mpd_client::tag::Tag;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::error;

#[derive(Debug)]
pub enum PlayerCommand {
    Previous,
    Toggle,
    Next,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Icons {
    /// Icon to display when playing.
    #[serde(default = "default_icon_play")]
    play: Option<String>,
    /// Icon to display when paused.
    #[serde(default = "default_icon_pause")]
    pause: Option<String>,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            pause: default_icon_pause(),
            play: default_icon_play(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MpdModule {
    /// TCP or Unix socket address.
    #[serde(default = "default_socket")]
    host: String,
    /// Format of current song info to display on the bar.
    #[serde(default = "default_format")]
    format: String,

    /// Player state icons
    #[serde(default)]
    icons: Icons,

    /// Path to root of music directory.
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

#[derive(Clone, Debug)]
pub struct SongUpdate {
    song: Song,
    status: Status,
    display_string: String,
}

impl Module<Button> for MpdModule {
    type SendMessage = Option<SongUpdate>;
    type ReceiveMessage = PlayerCommand;

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let host1 = self.host.clone();
        let host2 = self.host.clone();
        let format = self.format.clone();
        let icons = self.icons.clone();

        let re = Regex::new(r"\{([\w-]+)}")?;
        let tokens = get_tokens(&re, self.format.as_str());

        // poll mpd server
        spawn(async move {
            let client = get_client(&host1).await.expect("Failed to connect to MPD");
            let mut mpd_rx = client.subscribe();

            loop {
                let current_song = client.command(commands::CurrentSong).await;
                let status = client.command(commands::Status).await;

                if let (Ok(Some(song)), Ok(status)) = (current_song, status) {
                    let display_string =
                        replace_tokens(format.as_str(), &tokens, &song.song, &status, &icons);

                    let update = SongUpdate {
                        song: song.song,
                        status,
                        display_string,
                    };

                    tx.send(ModuleUpdateEvent::Update(Some(update))).await?;
                } else {
                    tx.send(ModuleUpdateEvent::Update(None)).await?;
                }

                // wait for player state change
                if mpd_rx.recv().await.is_err() {
                    break;
                }
            }

            Ok::<(), mpsc::error::SendError<ModuleUpdateEvent<Self::SendMessage>>>(())
        });

        // listen to ui events
        spawn(async move {
            let client = get_client(&host2).await?;

            while let Some(event) = rx.recv().await {
                let res = match event {
                    PlayerCommand::Previous => client.command(commands::Previous).await,
                    PlayerCommand::Toggle => match client.command(commands::Status).await {
                        Ok(status) => match status.state {
                            PlayState::Playing => client.command(commands::SetPause(true)).await,
                            PlayState::Paused => client.command(commands::SetPause(false)).await,
                            PlayState::Stopped => Ok(()),
                        },
                        Err(err) => Err(err),
                    },
                    PlayerCommand::Next => client.command(commands::Next).await,
                };

                if let Err(err) = res {
                    error!("Failed to send command to MPD server: {:?}", err);
                }
            }

            Ok::<(), MpdConnectionError>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<Button>> {
        let button = Button::new();
        let label = Label::new(None);
        label.set_angle(info.bar_position.get_angle());
        button.add(&label);

        let orientation = info.bar_position.get_orientation();

        button.connect_clicked(move |button| {
            context
                .tx
                .try_send(ModuleUpdateEvent::TogglePopup(Popup::button_pos(button, orientation)))
                .expect("Failed to send MPD popup open event");
        });

        {
            let button = button.clone();

            context.widget_rx.attach(None, move |mut event| {
                if let Some(event) = event.take() {
                    label.set_label(&event.display_string);
                    button.show();
                } else {
                    button.hide();
                }

                Continue(true)
            });
        };

        let popup = self.into_popup(context.controller_tx, context.popup_rx);

        Ok(ModuleWidget {
            widget: button,
            popup,
        })
    }

    fn into_popup(
        self,
        tx: Sender<Self::ReceiveMessage>,
        rx: glib::Receiver<Self::SendMessage>,
    ) -> Option<gtk::Box> {
        let container = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .name("popup-mpd")
            .build();

        let album_image = Image::builder()
            .width_request(128)
            .height_request(128)
            .name("album-art")
            .build();

        let info_box = gtk::Box::new(Orientation::Vertical, 10);
        let title_label = IconLabel::new("\u{f886}", None);
        let album_label = IconLabel::new("\u{f524}", None);
        let artist_label = IconLabel::new("\u{fd01}", None);

        title_label.container.set_widget_name("title");
        album_label.container.set_widget_name("album");
        artist_label.container.set_widget_name("label");

        info_box.add(&title_label.container);
        info_box.add(&album_label.container);
        info_box.add(&artist_label.container);

        let controls_box = gtk::Box::builder().name("controls").build();
        let btn_prev = Button::builder().label("\u{f9ad}").name("btn-prev").build();
        let btn_play_pause = Button::builder().label("").name("btn-play-pause").build();
        let btn_next = Button::builder().label("\u{f9ac}").name("btn-next").build();

        controls_box.add(&btn_prev);
        controls_box.add(&btn_play_pause);
        controls_box.add(&btn_next);

        info_box.add(&controls_box);

        container.add(&album_image);
        container.add(&info_box);

        let tx_prev = tx.clone();
        btn_prev.connect_clicked(move |_| {
            tx_prev
                .try_send(PlayerCommand::Previous)
                .expect("Failed to send prev track message");
        });

        let tx_toggle = tx.clone();
        btn_play_pause.connect_clicked(move |_| {
            tx_toggle
                .try_send(PlayerCommand::Toggle)
                .expect("Failed to send play/pause track message");
        });

        let tx_next = tx;
        btn_next.connect_clicked(move |_| {
            tx_next
                .try_send(PlayerCommand::Next)
                .expect("Failed to send next track message");
        });

        container.show_all();

        {
            let music_dir = self.music_dir;

            rx.attach(None, move |update| {
                if let Some(update) = update {
                    let prev_album = album_label.label.text();
                    let curr_album = update.song.album().unwrap_or_default();

                    // only update art when album changes
                    if prev_album != curr_album {
                        let cover_path = music_dir.join(
                            update
                                .song
                                .file_path()
                                .parent()
                                .expect("Song path should not be root")
                                .join("cover.jpg"),
                        );

                        if let Ok(pixbuf) = Pixbuf::from_file_at_scale(cover_path, 128, 128, true) {
                            album_image.set_from_pixbuf(Some(&pixbuf));
                        } else {
                            album_image.set_from_pixbuf(None);
                        }
                    }

                    title_label
                        .label
                        .set_text(update.song.title().unwrap_or_default());
                    album_label.label.set_text(curr_album);
                    artist_label
                        .label
                        .set_text(update.song.artists().first().unwrap_or(&String::new()));

                    match update.status.state {
                        PlayState::Stopped => {
                            btn_play_pause.set_sensitive(false);
                        }
                        PlayState::Playing => {
                            btn_play_pause.set_sensitive(true);
                            btn_play_pause.set_label("");
                        }
                        PlayState::Paused => {
                            btn_play_pause.set_sensitive(true);
                            btn_play_pause.set_label("");
                        }
                    }

                    let enable_prev = match update.status.current_song {
                        Some((pos, _)) => pos.0 > 0,
                        None => false,
                    };

                    let enable_next = match update.status.current_song {
                        Some((pos, _)) => pos.0 < update.status.playlist_length,
                        None => false,
                    };

                    btn_prev.set_sensitive(enable_prev);
                    btn_next.set_sensitive(enable_next);
                }

                Continue(true)
            });
        }

        Some(container)
    }
}

/// Replaces each of the formatting tokens in the formatting string
/// with actual data pulled from MPD
fn replace_tokens(
    format_string: &str,
    tokens: &Vec<String>,
    song: &Song,
    status: &Status,
    icons: &Icons,
) -> String {
    let mut compiled_string = format_string.to_string();
    for token in tokens {
        let value = get_token_value(song, status, icons, token);
        compiled_string =
            compiled_string.replace(format!("{{{}}}", token).as_str(), value.as_str());
    }
    compiled_string
}

/// Converts a string format token value
/// into its respective MPD value.
fn get_token_value(song: &Song, status: &Status, icons: &Icons, token: &str) -> String {
    let s = match token {
        "icon" => {
            let icon = match status.state {
                PlayState::Stopped => None,
                PlayState::Playing => icons.play.as_ref(),
                PlayState::Paused => icons.pause.as_ref(),
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

#[derive(Clone)]
struct IconLabel {
    label: Label,
    container: gtk::Box,
}

impl IconLabel {
    fn new(icon: &str, label: Option<&str>) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 5);

        let icon = Label::new(Some(icon));
        let label = Label::new(label);

        icon.style_context().add_class("icon");
        label.style_context().add_class("label");

        container.add(&icon);
        container.add(&label);

        Self { label, container }
    }
}
