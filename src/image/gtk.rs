use super::ImageProvider;
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image, Label, Orientation};
use std::ops::Deref;

#[cfg(any(feature = "music", feature = "workspaces", feature = "clipboard"))]
pub fn new_icon_button(input: &str, icon_theme: &IconTheme, size: i32) -> Button {
    let button = Button::new();

    if ImageProvider::is_definitely_image_input(input) {
        let image = Image::new();
        image.add_class("image");
        image.add_class("icon");

        match ImageProvider::parse(input, icon_theme, false, size)
            .map(|provider| provider.load_into_image(&image))
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

#[cfg(any(feature = "music", feature = "keys"))]
pub struct IconLabel {
    container: gtk::Box,
    label: Label,
    image: Image,

    icon_theme: IconTheme,
    size: i32,
}

#[cfg(any(feature = "music", feature = "keys"))]
impl IconLabel {
    pub fn new(input: &str, icon_theme: &IconTheme, size: i32) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let label = Label::builder().use_markup(true).build();
        label.add_class("icon");
        label.add_class("text-icon");

        let image = Image::new();
        image.add_class("icon");
        image.add_class("image");

        container.add(&image);
        container.add(&label);

        if ImageProvider::is_definitely_image_input(input) {
            ImageProvider::parse(input, icon_theme, false, size)
                .map(|provider| provider.load_into_image(&image));

            image.show();
        } else {
            label.set_text(input);
            label.show();
        }

        Self {
            container,
            label,
            image,
            icon_theme: icon_theme.clone(),
            size,
        }
    }

    pub fn set_label(&self, input: Option<&str>) {
        let label = &self.label;
        let image = &self.image;

        if let Some(input) = input {
            if ImageProvider::is_definitely_image_input(input) {
                ImageProvider::parse(input, &self.icon_theme, false, self.size)
                    .map(|provider| provider.load_into_image(image));

                label.hide();
                image.show();
            } else {
                label.set_label_escaped(input);

                image.hide();
                label.show();
            }
        } else {
            label.hide();
            image.hide();
        }
    }
}

impl Deref for IconLabel {
    type Target = gtk::Box;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}
