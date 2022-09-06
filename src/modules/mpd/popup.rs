pub use crate::popup::Popup;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{Button, Image, Label, Orientation};
use mpd_client::commands::responses::{PlayState, Song, Status};
use std::path::Path;
use tokio::sync::mpsc;

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

#[derive(Clone)]
pub struct MpdPopup {
    pub popup: Popup,

    cover: Image,

    title: IconLabel,
    album: IconLabel,
    artist: IconLabel,

    btn_prev: Button,
    btn_play_pause: Button,
    btn_next: Button,
}

#[derive(Debug)]
pub enum PopupEvent {
    Previous,
    Toggle,
    Next,
}

impl MpdPopup {
    pub fn new(popup: Popup, tx: mpsc::Sender<PopupEvent>) -> Self {
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

        popup.container.add(&album_image);
        popup.container.add(&info_box);

        let tx_prev = tx.clone();
        btn_prev.connect_clicked(move |_| {
            tx_prev
                .try_send(PopupEvent::Previous)
                .expect("Failed to send prev track message");
        });

        let tx_toggle = tx.clone();
        btn_play_pause.connect_clicked(move |_| {
            tx_toggle
                .try_send(PopupEvent::Toggle)
                .expect("Failed to send play/pause track message");
        });

        let tx_next = tx;
        btn_next.connect_clicked(move |_| {
            tx_next
                .try_send(PopupEvent::Next)
                .expect("Failed to send next track message");
        });

        Self {
            popup,
            cover: album_image,
            artist: artist_label,
            album: album_label,
            title: title_label,
            btn_prev,
            btn_play_pause,
            btn_next,
        }
    }

    // TODO: Use a channel instead of this method
    pub fn update(&self, song: &Song, status: &Status, path: &Path) {
        let prev_album = self.album.label.text();
        let curr_album = song.album().unwrap_or_default();

        // only update art when album changes
        if prev_album != curr_album {
            let cover_path = path.join(
                song.file_path()
                    .parent()
                    .expect("Song path should not be root")
                    .join("cover.jpg"),
            );

            if let Ok(pixbuf) = Pixbuf::from_file_at_scale(cover_path, 128, 128, true) {
                self.cover.set_from_pixbuf(Some(&pixbuf));
            }
        }

        self.title.label.set_text(song.title().unwrap_or_default());
        self.album.label.set_text(song.album().unwrap_or_default());
        self.artist
            .label
            .set_text(song.artists().first().unwrap_or(&String::new()));

        match status.state {
            PlayState::Stopped => {
                self.btn_play_pause.set_sensitive(false);
            }
            PlayState::Playing => {
                self.btn_play_pause.set_sensitive(true);
                self.btn_play_pause.set_label("");
            }
            PlayState::Paused => {
                self.btn_play_pause.set_sensitive(true);
                self.btn_play_pause.set_label("");
            }
        }

        let enable_prev = match status.current_song {
            Some((pos, _)) => pos.0 > 0,
            None => false,
        };

        let enable_next = match status.current_song {
            Some((pos, _)) => pos.0 < status.playlist_length,
            None => false,
        };

        self.btn_prev.set_sensitive(enable_prev);
        self.btn_next.set_sensitive(enable_next);
    }
}
