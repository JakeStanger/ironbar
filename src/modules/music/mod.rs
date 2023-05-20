mod config;

use crate::clients::music::{self, MusicClient, PlayerState, PlayerUpdate, Status, Track};
use crate::gtk_helpers::add_class;
use crate::image::{new_icon_button, new_icon_label, ImageProvider};
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::Popup;
use crate::{send_async, try_send};
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Label, Orientation, Scale};
use regex::Regex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::error;

pub use self::config::MusicModule;
use self::config::PlayerType;

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
                                    let display_string =
                                        replace_tokens(format.as_str(), &tokens, &track, &status);

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
                        PlayerCommand::Volume(vol) => client.set_volume_percent(vol),
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
        let button_contents = gtk::Box::new(Orientation::Horizontal, 5);
        add_class(&button_contents, "contents");

        button.add(&button_contents);

        let icon_play = new_icon_label(&self.icons.play, info.icon_theme, self.icon_size);
        let icon_pause = new_icon_label(&self.icons.pause, info.icon_theme, self.icon_size);
        let label = Label::new(None);

        label.set_angle(info.bar_position.get_angle());

        if let Some(truncate) = self.truncate {
            truncate.truncate_label(&label);
        }

        button_contents.add(&icon_pause);
        button_contents.add(&icon_play);
        button_contents.add(&label);

        let orientation = info.bar_position.get_orientation();

        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                try_send!(
                    tx,
                    ModuleUpdateEvent::TogglePopup(Popup::widget_geometry(button, orientation,))
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

                    match event.status.state {
                        PlayerState::Playing if self.show_status_icon => {
                            icon_play.show();
                            icon_pause.hide();
                        }
                        PlayerState::Paused if self.show_status_icon => {
                            icon_pause.show();
                            icon_play.hide();
                        }
                        PlayerState::Stopped => {
                            button.hide();
                        }
                        _ => {}
                    }

                    if !self.show_status_icon {
                        icon_pause.hide();
                        icon_play.hide();
                    }
                } else {
                    button.hide();
                    try_send!(tx, ModuleUpdateEvent::ClosePopup);
                }

                Continue(true)
            });
        };

        let popup = self.into_popup(context.controller_tx, context.popup_rx, info);

        Ok(ModuleWidget {
            widget: button,
            popup,
        })
    }

    fn into_popup(
        self,
        tx: Sender<Self::ReceiveMessage>,
        rx: glib::Receiver<Self::SendMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let icon_theme = info.icon_theme;

        let container = gtk::Box::new(Orientation::Horizontal, 10);

        let album_image = gtk::Image::builder()
            .width_request(128)
            .height_request(128)
            .build();
        add_class(&album_image, "album-art");

        let icons = self.icons;

        let info_box = gtk::Box::new(Orientation::Vertical, 10);
        let title_label = IconLabel::new(&icons.track, None, icon_theme);
        let album_label = IconLabel::new(&icons.album, None, icon_theme);
        let artist_label = IconLabel::new(&icons.artist, None, icon_theme);

        add_class(&title_label.container, "title");
        add_class(&album_label.container, "album");
        add_class(&artist_label.container, "artist");

        info_box.add(&title_label.container);
        info_box.add(&album_label.container);
        info_box.add(&artist_label.container);

        let controls_box = gtk::Box::new(Orientation::Horizontal, 0);
        add_class(&controls_box, "controls");

        let btn_prev = new_icon_button(&icons.prev, icon_theme, self.icon_size);
        add_class(&btn_prev, "btn-prev");

        let btn_play = new_icon_button(&icons.play, icon_theme, self.icon_size);
        add_class(&btn_play, "btn-play");

        let btn_pause = new_icon_button(&icons.pause, icon_theme, self.icon_size);
        add_class(&btn_pause, "btn-pause");

        let btn_next = new_icon_button(&icons.next, icon_theme, self.icon_size);
        add_class(&btn_next, "btn-next");

        controls_box.add(&btn_prev);
        controls_box.add(&btn_play);
        controls_box.add(&btn_pause);
        controls_box.add(&btn_next);

        info_box.add(&controls_box);

        let volume_box = gtk::Box::new(Orientation::Vertical, 5);
        add_class(&volume_box, "volume");

        let volume_slider = Scale::with_range(Orientation::Vertical, 0.0, 100.0, 5.0);
        volume_slider.set_inverted(true);
        add_class(&volume_slider, "slider");

        let volume_icon = new_icon_label(&icons.volume, icon_theme, self.icon_size);
        add_class(&volume_icon, "icon");

        volume_box.pack_start(&volume_slider, true, true, 0);
        volume_box.pack_end(&volume_icon, false, false, 0);

        container.add(&album_image);
        container.add(&info_box);
        container.add(&volume_box);

        let tx_prev = tx.clone();
        btn_prev.connect_clicked(move |_| {
            try_send!(tx_prev, PlayerCommand::Previous);
        });

        let tx_play = tx.clone();
        btn_play.connect_clicked(move |_| {
            try_send!(tx_play, PlayerCommand::Play);
        });

        let tx_pause = tx.clone();
        btn_pause.connect_clicked(move |_| {
            try_send!(tx_pause, PlayerCommand::Pause);
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
            let icon_theme = icon_theme.clone();
            let image_size = self.cover_image_size;

            let mut prev_cover = None;
            rx.attach(None, move |update| {
                if let Some(update) = update {
                    // only update art when album changes
                    let new_cover = update.song.cover_path;
                    if prev_cover != new_cover {
                        prev_cover = new_cover.clone();
                        let res = if let Some(image) = new_cover.and_then(|cover_path| {
                            ImageProvider::parse(&cover_path, &icon_theme, image_size)
                        }) {
                            image.load_into_image(album_image.clone())
                        } else {
                            album_image.set_from_pixbuf(None);
                            Ok(())
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
                            btn_pause.hide();
                            btn_play.show();
                            btn_play.set_sensitive(false);
                        }
                        PlayerState::Playing => {
                            btn_play.set_sensitive(false);
                            btn_play.hide();

                            btn_pause.set_sensitive(true);
                            btn_pause.show();
                        }
                        PlayerState::Paused => {
                            btn_pause.set_sensitive(false);
                            btn_pause.hide();

                            btn_play.set_sensitive(true);
                            btn_play.show();
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
) -> String {
    let mut compiled_string = format_string.to_string();
    for token in tokens {
        let value = get_token_value(song, status, token);
        compiled_string = compiled_string.replace(format!("{{{token}}}").as_str(), value.as_str());
    }
    compiled_string
}

/// Converts a string format token value
/// into its respective value.
fn get_token_value(song: &Track, status: &Status, token: &str) -> String {
    match token {
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

#[derive(Clone, Debug)]
struct IconLabel {
    label: Label,
    container: gtk::Box,
}

impl IconLabel {
    fn new(icon_input: &str, label: Option<&str>, icon_theme: &IconTheme) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 5);

        let icon = new_icon_label(icon_input, icon_theme, 24);
        let label = Label::new(label);

        add_class(&icon, "icon");
        add_class(&label, "label");

        container.add(&icon);
        container.add(&label);

        Self { label, container }
    }
}
