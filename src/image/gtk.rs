use super::ImageProvider;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image, Label, Orientation};
use tracing::error;

#[cfg(any(feature = "music", feature = "workspaces"))]
pub fn new_icon_button(input: &str, icon_theme: &IconTheme, size: i32) -> Button {
    let button = Button::new();

    if ImageProvider::is_definitely_image_input(input) {
        let image = Image::new();
        match ImageProvider::parse(input, icon_theme, size)
            .and_then(|provider| provider.load_into_image(image.clone()))
        {
            Ok(_) => {
                button.set_image(Some(&image));
                button.set_always_show_image(true);
            }
            Err(err) => {
                error!("{err:?}");
                button.set_label(input);
            }
        }
    } else {
        button.set_label(input);
    }

    button
}

#[cfg(feature = "music")]
pub fn new_icon_label(input: &str, icon_theme: &IconTheme, size: i32) -> gtk::Box {
    let container = gtk::Box::new(Orientation::Horizontal, 0);

    if ImageProvider::is_definitely_image_input(input) {
        let image = Image::new();
        container.add(&image);

        if let Err(err) = ImageProvider::parse(input, icon_theme, size)
            .and_then(|provider| provider.load_into_image(image))
        {
            error!("{err:?}");
        }
    } else {
        let label = Label::new(Some(input));
        container.add(&label);
    }

    container
}
