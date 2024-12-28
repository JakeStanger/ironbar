use std::cell::RefMut;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result;
use glib::{Propagation, PropertySet};
use gtk::prelude::*;
use gtk::{Button, IconTheme, Label, Orientation, Scale};
use regex::Regex;
use tokio::sync::{broadcast, mpsc};
use tracing::error;

pub use self::config::MusicModule;
use self::config::PlayerType;
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::music::{
    self, MusicClient, PlayerState, PlayerUpdate, ProgressTick, Status, Track,
};
use crate::clients::Clients;
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::image::{new_icon_button, IconLabel, ImageProvider};
use crate::modules::PopupButton;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::{module_impl, spawn};

mod config;

#[derive(Debug)]
pub enum PlayerCommand {
    Previous,
    Play,
    Pause,
    Next,
    Volume(u8),
    Seek(Duration),
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
pub enum ControllerEvent {
    Update(Option<SongUpdate>),
    UpdateProgress(ProgressTick),
}

#[derive(Clone, Debug)]
pub struct SongUpdate {
    song: Track,
    status: Status,
    display_string: String,
}

fn get_client(
    mut clients: RefMut<'_, Clients>,
    player_type: PlayerType,
    host: String,
    music_dir: PathBuf,
) -> Arc<dyn MusicClient> {
    let client_type = match player_type {
        PlayerType::Mpd => music::ClientType::Mpd { host, music_dir },
        PlayerType::Mpris => music::ClientType::Mpris,
    };

    clients.music(client_type)
}

impl Module<Button> for MusicModule {
    type SendMessage = ControllerEvent;
    type ReceiveMessage = PlayerCommand;

    module_impl!("music");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let format = self.format.clone();

        let re = Regex::new(r"\{([\w-]+)}")?;
        let tokens = get_tokens(&re, self.format.as_str());

        let client = get_client(
            context.ironbar.clients.borrow_mut(),
            self.player_type,
            self.host.clone(),
            self.music_dir.clone(),
        );

        // receive player updates
        {
            let tx = context.tx.clone();
            let client = client.clone();

            spawn(async move {
                loop {
                    let mut rx = client.subscribe_change();

                    while let Ok(update) = rx.recv().await {
                        match update {
                            PlayerUpdate::Update(track, status) => match *track {
                                Some(track) => {
                                    let display_string =
                                        replace_tokens(format.as_str(), &tokens, &track);

                                    let update = SongUpdate {
                                        song: track,
                                        status,
                                        display_string,
                                    };

                                    tx.send_update(ControllerEvent::Update(Some(update))).await;
                                }
                                None => tx.send_update(ControllerEvent::Update(None)).await,
                            },
                            PlayerUpdate::ProgressTick(progress_tick) => {
                                tx.send_update(ControllerEvent::UpdateProgress(progress_tick))
                                    .await;
                            }
                        }
                    }
                }
            });
        }

        // listen to ui events
        {
            spawn(async move {
                while let Some(event) = rx.recv().await {
                    let res = match event {
                        PlayerCommand::Previous => client.prev(),
                        PlayerCommand::Play => client.play(),
                        PlayerCommand::Pause => client.pause(),
                        PlayerCommand::Next => client.next(),
                        PlayerCommand::Volume(vol) => client.set_volume_percent(vol),
                        PlayerCommand::Seek(duration) => client.seek(duration),
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
    ) -> Result<ModuleParts<Button>> {
        let button = Button::new();
        let button_contents = gtk::Box::new(Orientation::Horizontal, 5);
        button_contents.add_class("contents");

        button.add(&button_contents);

        let icon_play = IconLabel::new(&self.icons.play, info.icon_theme, self.icon_size);
        let icon_pause = IconLabel::new(&self.icons.pause, info.icon_theme, self.icon_size);

        let label = Label::builder()
            .use_markup(true)
            .angle(info.bar_position.get_angle())
            .build();

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        button_contents.add(icon_pause.deref());
        button_contents.add(icon_play.deref());
        button_contents.add(&label);

        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
            });
        }

        {
            let button = button.clone();

            let tx = context.tx.clone();

            context.subscribe().recv_glib(move |event| {
                let ControllerEvent::Update(mut event) = event else {
                    return;
                };

                if let Some(event) = event.take() {
                    label.set_label_escaped(&event.display_string);

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
                    tx.send_spawn(ModuleUpdateEvent::ClosePopup);
                }
            });
        };

        let rx = context.subscribe();
        let popup = self
            .into_popup(context.controller_tx.clone(), rx, context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let icon_theme = info.icon_theme;

        let container = gtk::Box::new(Orientation::Vertical, 10);
        let main_container = gtk::Box::new(Orientation::Horizontal, 10);

        let album_image = gtk::Image::builder()
            .width_request(128)
            .height_request(128)
            .build();
        album_image.add_class("album-art");

        let icons = self.icons;

        let info_box = gtk::Box::new(Orientation::Vertical, 10);
        let title_label = IconPrefixedLabel::new(&icons.track, None, icon_theme);
        let album_label = IconPrefixedLabel::new(&icons.album, None, icon_theme);
        let artist_label = IconPrefixedLabel::new(&icons.artist, None, icon_theme);

        title_label.container.add_class("title");
        album_label.container.add_class("album");
        artist_label.container.add_class("artist");

        info_box.add(&title_label.container);
        info_box.add(&album_label.container);
        info_box.add(&artist_label.container);

        let controls_box = gtk::Box::new(Orientation::Horizontal, 0);
        controls_box.add_class("controls");

        let btn_prev = new_icon_button(&icons.prev, icon_theme, self.icon_size);
        btn_prev.add_class("btn-prev");

        let btn_play = new_icon_button(&icons.play, icon_theme, self.icon_size);
        btn_play.add_class("btn-play");

        let btn_pause = new_icon_button(&icons.pause, icon_theme, self.icon_size);
        btn_pause.add_class("btn-pause");

        let btn_next = new_icon_button(&icons.next, icon_theme, self.icon_size);
        btn_next.add_class("btn-next");

        controls_box.add(&btn_prev);
        controls_box.add(&btn_play);
        controls_box.add(&btn_pause);
        controls_box.add(&btn_next);

        info_box.add(&controls_box);

        let volume_box = gtk::Box::new(Orientation::Vertical, 5);
        volume_box.add_class("volume");

        let volume_slider = Scale::with_range(Orientation::Vertical, 0.0, 100.0, 5.0);
        volume_slider.set_inverted(true);
        volume_slider.add_class("slider");

        let volume_icon = IconLabel::new(&icons.volume, icon_theme, self.icon_size);
        volume_icon.add_class("icon");

        volume_box.pack_start(&volume_slider, true, true, 0);
        volume_box.pack_end(volume_icon.deref(), false, false, 0);

        main_container.add(&album_image);
        main_container.add(&info_box);
        main_container.add(&volume_box);
        container.add(&main_container);

        let tx_prev = tx.clone();
        btn_prev.connect_clicked(move |_| {
            tx_prev.send_spawn(PlayerCommand::Previous);
        });

        let tx_play = tx.clone();
        btn_play.connect_clicked(move |_| {
            tx_play.send_spawn(PlayerCommand::Play);
        });

        let tx_pause = tx.clone();
        btn_pause.connect_clicked(move |_| {
            tx_pause.send_spawn(PlayerCommand::Pause);
        });

        let tx_next = tx.clone();
        btn_next.connect_clicked(move |_| {
            tx_next.send_spawn(PlayerCommand::Next);
        });

        let tx_vol = tx.clone();
        volume_slider.connect_change_value(move |_, _, val| {
            tx_vol.send_spawn(PlayerCommand::Volume(val as u8));
            Propagation::Proceed
        });

        let progress_box = gtk::Box::new(Orientation::Horizontal, 5);
        progress_box.add_class("progress");

        let progress_label = Label::new(None);
        progress_label.add_class("label");

        let progress = Scale::builder()
            .orientation(Orientation::Horizontal)
            .draw_value(false)
            .hexpand(true)
            .build();
        progress.add_class("slider");

        progress_box.add(&progress);
        progress_box.add(&progress_label);
        container.add(&progress_box);

        let drag_lock = Arc::new(AtomicBool::new(false));
        {
            let drag_lock = drag_lock.clone();
            progress.connect_button_press_event(move |_, _| {
                drag_lock.set(true);
                Propagation::Proceed
            });
        }

        {
            let drag_lock = drag_lock.clone();
            progress.connect_button_release_event(move |scale, _| {
                let value = scale.value();
                tx.send_spawn(PlayerCommand::Seek(Duration::from_secs_f64(value)));

                drag_lock.set(false);
                Propagation::Proceed
            });
        }

        container.show_all();

        {
            let icon_theme = icon_theme.clone();
            let image_size = self.cover_image_size;

            let mut prev_cover = None;
            rx.recv_glib(move |event| {
                match event {
                    ControllerEvent::Update(Some(update)) => {
                        // only update art when album changes
                        let new_cover = update.song.cover_path;
                        if prev_cover != new_cover {
                            prev_cover.clone_from(&new_cover);
                            let res = if let Some(image) = new_cover.and_then(|cover_path| {
                                ImageProvider::parse(&cover_path, &icon_theme, false, image_size)
                            }) {
                                album_image.show();
                                image.load_into_image(&album_image)
                            } else {
                                album_image.set_from_pixbuf(None);
                                album_image.hide();
                                Ok(())
                            };

                            if let Err(err) = res {
                                error!("{err:?}");
                            }
                        }

                        update_popup_metadata_label(update.song.title, &title_label);
                        update_popup_metadata_label(update.song.album, &album_label);
                        update_popup_metadata_label(update.song.artist, &artist_label);

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

                        if let Some(volume) = update.status.volume_percent {
                            volume_slider.set_value(f64::from(volume));
                            volume_box.show();
                        } else {
                            volume_box.hide();
                        }
                    }
                    ControllerEvent::UpdateProgress(progress_tick)
                        if !drag_lock.load(Ordering::Relaxed) =>
                    {
                        if let (Some(elapsed), Some(duration)) =
                            (progress_tick.elapsed, progress_tick.duration)
                        {
                            progress_label.set_label_escaped(&format!(
                                "{}/{}",
                                format_time(elapsed),
                                format_time(duration)
                            ));

                            progress.set_value(elapsed.as_secs_f64());
                            progress.set_range(0.0, duration.as_secs_f64());
                            progress_box.show_all();
                        } else {
                            progress_box.hide();
                        }
                    }
                    _ => {}
                };
            });
        }

        Some(container)
    }
}

fn update_popup_metadata_label(text: Option<String>, label: &IconPrefixedLabel) {
    match text {
        Some(value) => {
            label.label.set_label_escaped(&value);
            label.container.show_all();
        }
        None => {
            label.container.hide();
        }
    }
}

/// Replaces each of the formatting tokens in the formatting string
/// with actual data pulled from the music player
fn replace_tokens(format_string: &str, tokens: &Vec<String>, song: &Track) -> String {
    let mut compiled_string = format_string.to_string();
    for token in tokens {
        let value = get_token_value(song, token);
        compiled_string = compiled_string.replace(format!("{{{token}}}").as_str(), value.as_str());
    }
    compiled_string
}

/// Converts a string format token value
/// into its respective value.
fn get_token_value(song: &Track, token: &str) -> String {
    match token {
        "title" => song.title.clone(),
        "album" => song.album.clone(),
        "artist" => song.artist.clone(),
        "date" => song.date.clone(),
        "disc" => song.disc.map(|x| x.to_string()),
        "genre" => song.genre.clone(),
        "track" => song.track.map(|x| x.to_string()),
        _ => Some(token.to_string()),
    }
    .unwrap_or_default()
}

#[derive(Clone, Debug)]
struct IconPrefixedLabel {
    label: Label,
    container: gtk::Box,
}

impl IconPrefixedLabel {
    fn new(icon_input: &str, label: Option<&str>, icon_theme: &IconTheme) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 5);

        let icon = IconLabel::new(icon_input, icon_theme, 24);

        let mut builder = Label::builder().use_markup(true);

        if let Some(label) = label {
            builder = builder.label(label);
        }

        let label = builder.build();

        icon.add_class("icon-box");
        label.add_class("label");

        container.add(icon.deref());
        container.add(&label);

        Self { label, container }
    }
}
