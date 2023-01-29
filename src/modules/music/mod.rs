mod config;

use std::path::PathBuf;
use crate::clients::music::{self, MusicClient, PlayerState, PlayerUpdate, Status, Track};
use crate::image::ImageProvider;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::Popup;
use crate::{send_async, try_send};
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Label, Orientation, Scale};
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::error;

pub use self::config::MusicModule;
use self::config::{Icons, PlayerType};

#[derive(Debug)]
pub enum PlayerCommand {
    Previous,
    Play,
    Pause,
    Next,
    Volume(u8),
}

/// Formats a duration given in seconds
/// in hh:mm format
fn format_time(duration: Duration) -> String {
    let time = duration.as_secs();
    let minutes = (time / 60) % 60;
    let seconds = time % 60;

    format!("{minutes:0>2}:{seconds:0>2}")
}

/// Extracts the formatting tokens from a formatting string
fn get_tokens(re: &Regex, format_string: &str) -> Vec<String> {
    re.captures_iter(format_string)
        .map(|caps| caps[1].to_string())
        .collect::<Vec<_>>()
}

#[derive(Clone, Debug)]
pub struct SongUpdate {
    song: Track,
    status: Status,
    display_string: String,
}

async fn get_client(
    player_type: PlayerType,
    host: &str,
    music_dir: PathBuf,
) -> Box<Arc<dyn MusicClient>> {
    match player_type {
        PlayerType::Mpd => music::get_client(music::ClientType::Mpd { host, music_dir }),
        PlayerType::Mpris => music::get_client(music::ClientType::Mpris {}),
    }
    .await
}

impl Module<Button> for MusicModule {
    type SendMessage = Option<SongUpdate>;
    type ReceiveMessage = PlayerCommand;

    fn name() -> &'static str {
        "music"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let format = self.format.clone();
        let icons = self.icons.clone();

        let re = Regex::new(r"\{([\w-]+)}")?;
        let tokens = get_tokens(&re, self.format.as_str());

        // receive player updates
        {
            let player_type = self.player_type;
            let host = self.host.clone();
            let music_dir = self.music_dir.clone();

            spawn(async move {
                loop {
                    let mut rx = {
                        let client = get_client(player_type, &host, music_dir.clone()).await;
                        client.subscribe_change()
                    };

                    while let Ok(update) = rx.recv().await {
                        match update {
                            PlayerUpdate::Update(track, status) => match *track {
                                Some(track) => {
                                    let display_string = replace_tokens(
                                        format.as_str(),
                                        &tokens,
                                        &track,
                                        &status,
                                        &icons,
                                    );

                                    let update = SongUpdate {
                                        song: track,
                                        status,
                                        display_string,
                                    };

                                    send_async!(tx, ModuleUpdateEvent::Update(Some(update)));
                                }
                                None => send_async!(tx, ModuleUpdateEvent::Update(None)),
                            },
                            PlayerUpdate::Disconnect => break,
                        }
                    }
                }
            });
        }

        // listen to ui events
        {
            let player_type = self.player_type;
            let host = self.host.clone();
            let music_dir = self.music_dir.clone();

            spawn(async move {
                while let Some(event) = rx.recv().await {
                    let client = get_client(player_type, &host, music_dir.clone()).await;
                    let res = match event {
                        PlayerCommand::Previous => client.prev(),
                        PlayerCommand::Play => client.play(),
                        PlayerCommand::Pause => client.pause(),
                        PlayerCommand::Next => client.next(),
                        PlayerCommand::Volume(vol) => client.set_volume_percent(vol), // .unwrap_or_else(|_| error!("Failed to update player volume")),
                    };

                    if let Err(err) = res {
                        error!("Failed to send command to server: {:?}", err);
                    }
                }
            });
        }

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

        if let Some(truncate) = self.truncate {
            truncate.truncate_label(&label);
        }

        button.add(&label);

        let orientation = info.bar_position.get_orientation();

        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                try_send!(
                    tx,
                    ModuleUpdateEvent::TogglePopup(Popup::button_pos(button, orientation,))
                );
            });
        }

        {
            let button = button.clone();
            let tx = context.tx.clone();

            context.widget_rx.attach(None, move |mut event| {
                if let Some(event) = event.take() {
                    label.set_label(&event.display_string);
                    button.show();
                } else {
                    button.hide();
                    try_send!(tx, ModuleUpdateEvent::ClosePopup);
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
        let icon_theme = IconTheme::new();

        let container = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .name("popup-music")
            .build();

        let album_image = gtk::Image::builder()
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
        artist_label.container.set_widget_name("artist");

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

        let volume_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(5)
            .name("volume")
            .build();

        let volume_slider = Scale::with_range(Orientation::Vertical, 0.0, 100.0, 5.0);
        volume_slider.set_inverted(true);
        volume_slider.set_widget_name("slider");

        let volume_icon = Label::new(Some(&self.icons.volume));
        volume_icon.style_context().add_class("icon");

        volume_box.pack_start(&volume_slider, true, true, 0);
        volume_box.pack_end(&volume_icon, false, false, 0);

        container.add(&album_image);
        container.add(&info_box);
        container.add(&volume_box);

        let tx_prev = tx.clone();
        btn_prev.connect_clicked(move |_| {
            try_send!(tx_prev, PlayerCommand::Previous);
        });

        let tx_toggle = tx.clone();
        btn_play_pause.connect_clicked(move |button| {
            if button.style_context().has_class("playing") {
                try_send!(tx_toggle, PlayerCommand::Pause);
            } else {
                try_send!(tx_toggle, PlayerCommand::Play);
            }
        });

        let tx_next = tx.clone();
        btn_next.connect_clicked(move |_| {
            try_send!(tx_next, PlayerCommand::Next);
        });

        let tx_vol = tx;
        volume_slider.connect_change_value(move |_, _, val| {
            try_send!(tx_vol, PlayerCommand::Volume(val as u8));
            Inhibit(false)
        });

        container.show_all();

        {
            let mut prev_cover = None;
            rx.attach(None, move |update| {
                if let Some(update) = update {
                    // only update art when album changes
                    let new_cover = update.song.cover_path;
                    if prev_cover != new_cover {
                        prev_cover = new_cover.clone();
                        let res = match new_cover.map(|cover_path| ImageProvider::parse(cover_path, &icon_theme, 128))
                        {
                            Some(Ok(image)) => image.load_into_image(album_image.clone()),
                            Some(Err(err)) => {
                                album_image.set_from_pixbuf(None);
                                Err(err)
                            }
                            None => Ok(album_image.set_from_pixbuf(None)),
                        };
                        if let Err(err) = res {
                            error!("{err:?}");
                        }
                    }

                    title_label
                        .label
                        .set_text(&update.song.title.unwrap_or_default());
                    album_label
                        .label
                        .set_text(&update.song.album.unwrap_or_default());
                    artist_label
                        .label
                        .set_text(&update.song.artist.unwrap_or_default());

                    match update.status.state {
                        PlayerState::Stopped => {
                            btn_play_pause.set_sensitive(false);
                        }
                        PlayerState::Playing => {
                            btn_play_pause.set_sensitive(true);
                            btn_play_pause.set_label(&self.icons.pause);

                            let style_context = btn_play_pause.style_context();
                            style_context.add_class("playing");
                            style_context.remove_class("paused");
                        }
                        PlayerState::Paused => {
                            btn_play_pause.set_sensitive(true);
                            btn_play_pause.set_label(&self.icons.play);

                            let style_context = btn_play_pause.style_context();
                            style_context.add_class("paused");
                            style_context.remove_class("playing");
                        }
                    }

                    let enable_prev = update.status.playlist_position > 0;

                    let enable_next =
                        update.status.playlist_position < update.status.playlist_length;

                    btn_prev.set_sensitive(enable_prev);
                    btn_next.set_sensitive(enable_next);

                    volume_slider.set_value(update.status.volume_percent as f64);
                }

                Continue(true)
            });
        }

        Some(container)
    }
}

/// Replaces each of the formatting tokens in the formatting string
/// with actual data pulled from the music player
fn replace_tokens(
    format_string: &str,
    tokens: &Vec<String>,
    song: &Track,
    status: &Status,
    icons: &Icons,
) -> String {
    let mut compiled_string = format_string.to_string();
    for token in tokens {
        let value = get_token_value(song, status, icons, token);
        compiled_string = compiled_string.replace(format!("{{{token}}}").as_str(), value.as_str());
    }
    compiled_string
}

/// Converts a string format token value
/// into its respective value.
fn get_token_value(song: &Track, status: &Status, icons: &Icons, token: &str) -> String {
    match token {
        "icon" => match status.state {
            PlayerState::Stopped => None,
            PlayerState::Playing => Some(&icons.play),
            PlayerState::Paused => Some(&icons.pause),
        }
        .map(std::string::ToString::to_string),
        "title" => song.title.clone(),
        "album" => song.album.clone(),
        "artist" => song.artist.clone(),
        "date" => song.date.clone(),
        "disc" => song.disc.map(|x| x.to_string()),
        "genre" => song.genre.clone(),
        "track" => song.track.map(|x| x.to_string()),
        "duration" => status.duration.map(format_time),
        "elapsed" => status.elapsed.map(format_time),
        _ => Some(token.to_string()),
    }
    .unwrap_or_default()
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
