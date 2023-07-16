use super::ImageProvider;
use crate::gtk_helpers::IronbarGtkExt;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image, Label, Orientation};

#[cfg(any(feature = "music", feature = "workspaces", feature = "clipboard"))]
pub fn new_icon_button(input: &str, icon_theme: &IconTheme, size: i32) -> Button {
    let button = Button::new();

    if ImageProvider::is_definitely_image_input(input) {
        let image = Image::new();
        image.add_class("image");
        image.add_class("icon");

        match ImageProvider::parse(input, icon_theme, size)
            .map(|provider| provider.load_into_image(image.clone()))
        {
            Some(_) => {
                button.set_image(Some(&image));
                button.set_always_show_image(true);
            }
            None => {
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
        image.add_class("icon");
        image.add_class("image");

        container.add(&image);

        ImageProvider::parse(input, icon_theme, size)
            .map(|provider| provider.load_into_image(image));
    } else {
        let label = Label::new(Some(input));
        label.add_class("icon");
        label.add_class("text-icon");

        container.add(&label);
    }

    container
}
