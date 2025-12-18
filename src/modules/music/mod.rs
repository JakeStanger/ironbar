use std::borrow::Cow;
use std::cell::RefMut;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use color_eyre::Result;
use glib::Propagation;
use gtk::gdk::Paintable;
use gtk::prelude::*;
use gtk::{Button, ContentFit, EventSequenceState, GestureClick, Label, Orientation, Scale};
use tokio::sync::mpsc;
use tracing::{error, warn};

pub use self::config::MusicModule;
use self::config::PlayerType;
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::Clients;
use crate::clients::music::{
    self, MusicClient, PlayerState, PlayerUpdate, ProgressTick, Status, Track,
};
use crate::gtk_helpers::{IronbarLabelExt, OverflowLabel};
use crate::image::{IconButton, IconLabel, IconPrefixedLabel};
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
/// in hh:mm:ss format
fn format_time(duration: Duration) -> String {
    let time = duration.as_secs();
    let hours = time / (60 * 60);
    let minutes = (time / 60) % 60;
    let seconds = time % 60;

    if hours > 0 {
        format!("{hours}:{minutes:0>2}:{seconds:0>2}")
    } else {
        format!("{minutes:0>2}:{seconds:0>2}")
    }
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
        #[cfg(feature = "music+mpd")]
        PlayerType::Mpd => music::ClientType::Mpd { host, music_dir },
        #[cfg(feature = "music+mpris")]
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
                                    let display_string = replace_tokens(format.as_str(), &track);

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
        let button_contents = gtk::Box::new(self.layout.orientation(info), 5);
        button_contents.add_css_class("contents");

        button.set_child(Some(&button_contents));

        let image_provider = context.ironbar.image_provider();

        let icon_play = IconLabel::new(&self.icons.play, self.icon_size, &image_provider);
        let icon_pause = IconLabel::new(&self.icons.pause, self.icon_size, &image_provider);

        icon_play.label().set_justify(self.layout.justify.into());

        icon_pause.label().set_justify(self.layout.justify.into());

        let label = OverflowLabel::new(
            Label::builder()
                .use_markup(true)
                .justify(self.layout.justify.into())
                .build(),
            self.truncate,
            self.marquee.clone(),
        );

        button_contents.append(&*icon_pause);
        button_contents.append(&*icon_play);
        button_contents.append(label.widget());

        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
            });
        }

        let rx = context.subscribe();

        rx.recv_glib(
            (&button, &context.tx, &label),
            move |(button, tx, label), event| {
                let ControllerEvent::Update(mut event) = event else {
                    return;
                };

                if let Some(event) = event.take() {
                    label.set_label_escaped(&event.display_string);

                    button.set_visible(true);

                    match event.status.state {
                        PlayerState::Playing if self.show_status_icon => {
                            icon_play.set_visible(true);
                            icon_pause.set_visible(false);
                        }
                        PlayerState::Paused if self.show_status_icon => {
                            icon_pause.set_visible(true);
                            icon_play.set_visible(false);
                        }
                        PlayerState::Stopped => {
                            button.set_visible(false);
                        }
                        _ => {}
                    }

                    if !self.show_status_icon {
                        icon_pause.set_visible(false);
                        icon_play.set_visible(false);
                    }
                } else {
                    button.set_visible(false);
                    tx.send_spawn(ModuleUpdateEvent::ClosePopup);
                }
            },
        );

        let popup = self
            .into_popup(context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let image_provider = context.ironbar.image_provider();

        let container = gtk::Box::new(Orientation::Vertical, 10);
        let main_container = gtk::Box::new(Orientation::Horizontal, 10);

        let album_image = gtk::Picture::builder()
            .content_fit(ContentFit::ScaleDown)
            .width_request(128)
            .height_request(128)
            .build();

        album_image.add_css_class("album-art");

        let icons = self.icons;

        let info_box = gtk::Box::new(Orientation::Vertical, 10);

        let title_overflow = OverflowLabel::new(
            Label::builder().use_markup(true).build(),
            self.truncate_popup_title,
            self.marquee_popup_title.clone(),
        );
        let title_label =
            IconPrefixedLabel::with_overflow(&icons.track, title_overflow, &image_provider);

        let album_label = IconPrefixedLabel::new(&icons.album, None, &image_provider);
        if let Some(truncate) = self.truncate_popup_album {
            album_label.label().truncate(truncate);
        }

        let artist_label = IconPrefixedLabel::new(&icons.artist, None, &image_provider);
        if let Some(truncate) = self.truncate_popup_artist {
            artist_label.label().truncate(truncate);
        }

        title_label.add_css_class("title");
        album_label.add_css_class("album");
        artist_label.add_css_class("artist");

        info_box.append(&*title_label);
        info_box.append(&*album_label);
        info_box.append(&*artist_label);

        let controls_box = gtk::Box::new(Orientation::Horizontal, 0);
        controls_box.add_css_class("controls");

        let btn_prev = IconButton::new(&icons.prev, self.icon_size, image_provider.clone());
        btn_prev.add_css_class("btn-prev");

        let btn_play = IconButton::new(&icons.play, self.icon_size, image_provider.clone());
        btn_play.add_css_class("btn-play");

        let btn_pause = IconButton::new(&icons.pause, self.icon_size, image_provider.clone());
        btn_pause.add_css_class("btn-pause");

        let btn_next = IconButton::new(&icons.next, self.icon_size, image_provider.clone());
        btn_next.add_css_class("btn-next");

        controls_box.append(&*btn_prev);
        controls_box.append(&*btn_play);
        controls_box.append(&*btn_pause);
        controls_box.append(&*btn_next);

        info_box.append(&controls_box);

        let volume_box = gtk::Box::new(Orientation::Vertical, 5);
        volume_box.add_css_class("volume");

        let volume_slider = Scale::with_range(Orientation::Vertical, 0.0, 100.0, 5.0);
        volume_slider.set_inverted(true);
        volume_slider.add_css_class("slider");

        let volume_icon = IconLabel::new(&icons.volume, self.icon_size, &image_provider);
        volume_icon.add_css_class("icon");

        volume_box.prepend(&volume_slider);
        volume_box.append(&*volume_icon);

        volume_slider.set_vexpand(true);

        main_container.append(&album_image);
        main_container.append(&info_box);
        main_container.append(&volume_box);
        container.append(&main_container);

        info_box.set_hexpand(true);

        let tx_prev = context.controller_tx.clone();
        btn_prev.connect_clicked(move |_| {
            tx_prev.send_spawn(PlayerCommand::Previous);
        });

        let tx_play = context.controller_tx.clone();
        btn_play.connect_clicked(move |_| {
            tx_play.send_spawn(PlayerCommand::Play);
        });

        let tx_pause = context.controller_tx.clone();
        btn_pause.connect_clicked(move |_| {
            tx_pause.send_spawn(PlayerCommand::Pause);
        });

        let tx_next = context.controller_tx.clone();
        btn_next.connect_clicked(move |_| {
            tx_next.send_spawn(PlayerCommand::Next);
        });

        let tx_vol = context.controller_tx.clone();
        volume_slider.connect_change_value(move |_, _, val| {
            tx_vol.send_spawn(PlayerCommand::Volume(val as u8));
            Propagation::Proceed
        });

        let progress_box = gtk::Box::new(Orientation::Horizontal, 5);
        progress_box.add_css_class("progress");

        let progress_label = Label::new(None);
        progress_label.add_css_class("label");

        let progress = Scale::builder()
            .orientation(Orientation::Horizontal)
            .draw_value(false)
            .hexpand(true)
            .build();
        progress.add_css_class("slider");

        progress_box.append(&progress);
        progress_box.append(&progress_label);
        container.append(&progress_box);

        let event_handler = GestureClick::new();
        let drag_lock = Arc::new(AtomicBool::new(false));

        {
            let drag_lock = drag_lock.clone();
            event_handler.connect_pressed(move |gesture, _, _, _| {
                gesture.set_state(EventSequenceState::Claimed);
                drag_lock.store(true, Ordering::Relaxed);
            });
        }

        {
            let drag_lock = drag_lock.clone();
            let scale = progress.clone();
            let tx = context.controller_tx.clone();
            event_handler.connect_released(move |gesture, _, _, _| {
                gesture.set_state(EventSequenceState::Claimed);

                let value = scale.value();
                tx.send_spawn(PlayerCommand::Seek(Duration::from_secs_f64(value)));

                drag_lock.store(false, Ordering::Relaxed);
            });
        }

        progress.add_controller(event_handler);

        let image_size = self.cover_image_size;

        let mut prev_cover = None;
        context.subscribe().recv_glib((), move |(), event| {
            match event {
                ControllerEvent::Update(Some(update)) => {
                    // only update art when album changes
                    let new_cover = update.song.cover_path;
                    if prev_cover != new_cover {
                        prev_cover.clone_from(&new_cover);

                        if let Some(cover_path) = new_cover {
                            let image_provider = image_provider.clone();
                            let album_image = album_image.clone();

                            glib::spawn_future_local(async move {
                                let success = match image_provider
                                    .load_into_picture(&cover_path, image_size, false, &album_image)
                                    .await
                                {
                                    Ok(true) => {
                                        album_image.set_visible(true);
                                        true
                                    }
                                    Ok(false) => {
                                        warn!("failed to parse image: {}", cover_path);
                                        false
                                    }
                                    Err(err) => {
                                        error!("failed to load image: {}", err);
                                        false
                                    }
                                };

                                if !success {
                                    album_image.set_paintable(None::<&Paintable>);
                                    album_image.set_visible(false);
                                }
                            });
                        } else {
                            album_image.set_paintable(None::<&Paintable>);
                            album_image.set_visible(false);
                        }
                    }

                    update_popup_metadata_label(update.song.title, &title_label);
                    update_popup_metadata_label(update.song.album, &album_label);
                    update_popup_metadata_label(update.song.artist, &artist_label);

                    match update.status.state {
                        PlayerState::Stopped => {
                            btn_pause.set_visible(false);
                            btn_play.set_visible(true);
                            btn_play.set_sensitive(false);
                        }
                        PlayerState::Playing => {
                            btn_play.set_sensitive(false);
                            btn_play.set_visible(false);

                            btn_pause.set_sensitive(true);
                            btn_pause.set_visible(true);
                        }
                        PlayerState::Paused => {
                            btn_pause.set_sensitive(false);
                            btn_pause.set_visible(false);

                            btn_play.set_sensitive(true);
                            btn_play.set_visible(true);
                        }
                    }

                    let enable_prev = update.status.playlist_position > 0;

                    let enable_next =
                        update.status.playlist_position < update.status.playlist_length;

                    btn_prev.set_sensitive(enable_prev);
                    btn_next.set_sensitive(enable_next);

                    if let Some(volume) = update.status.volume_percent {
                        volume_slider.set_value(f64::from(volume));
                        volume_box.set_visible(true);
                    } else {
                        volume_box.set_visible(false);
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
                        progress_box.set_visible(true);
                    } else {
                        progress_box.set_visible(false);
                    }
                }
                _ => {}
            }
        });

        Some(container)
    }
}

fn update_popup_metadata_label(text: Option<String>, label: &IconPrefixedLabel) {
    match text {
        Some(value) => {
            label.set_label_escaped(&value);
        }
        None => {
            label.set_visible(false);
        }
    }
}

/// Replaces each of the formatting tokens in the formatting string
/// with actual data pulled from the music player
fn replace_tokens<'a>(format_string: &'a str, song: &'a Track) -> String {
    format_string
        .replace_with("{title}", || song.title.clone().unwrap_or_default())
        .replace_with("{album}", || song.album.clone().unwrap_or_default())
        .replace_with("{artist}", || song.artist.clone().unwrap_or_default())
        .replace_with("{date}", || song.date.clone().unwrap_or_default())
        .replace_with("{disc}", || {
            song.disc.map(|val| val.to_string()).unwrap_or_default()
        })
        .replace_with("{genre}", || song.genre.clone().unwrap_or_default())
        .replace_with("{track}", || {
            song.track.map(|val| val.to_string()).unwrap_or_default()
        })
        .to_string()
}

trait StringExt<'a> {
    fn replace_with<F>(self, from: &str, to: F) -> Cow<'a, str>
    where
        F: Fn() -> String;
}

impl<'a> StringExt<'a> for &'a str {
    fn replace_with<F>(self, from: &str, to: F) -> Cow<'a, str>
    where
        F: Fn() -> String,
    {
        if self.contains(from) {
            Cow::Owned(self.replace(from, &to()))
        } else {
            Cow::Borrowed(self)
        }
    }
}
