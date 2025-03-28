use super::ImageProvider;
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image, Label, Orientation};
use std::ops::Deref;

#[derive(Debug, Clone)]
#[cfg(any(
    feature = "clipboard",
    feature = "keyboard",
    feature = "music",
    feature = "workspaces"
))]
pub struct IconButton {
    button: Button,
    label: Label,
}

#[cfg(any(
    feature = "clipboard",
    feature = "keyboard",
    feature = "music",
    feature = "workspaces"
))]
impl IconButton {
    pub fn new(input: &str, icon_theme: &IconTheme, size: i32) -> Self {
        let button = Button::new();
        let image = Image::new();
        let label = Label::new(Some(input));

        if ImageProvider::is_definitely_image_input(input) {
            image.add_class("image");
            image.add_class("icon");

            match ImageProvider::parse(input, icon_theme, false, size)
                .map(|provider| provider.load_into_image(&image))
            {
                Some(_) => {
                    button.set_child(Some(&image));
                }
                None => {
                    button.set_child(Some(&label));
                }
            }
        } else {
            button.set_child(Some(&label));
        }

        Self { button, label }
    }

    pub fn label(&self) -> &Label {
        &self.label
    }
}

impl Deref for IconButton {
    type Target = Button;

    fn deref(&self) -> &Self::Target {
        &self.button
    }
}

#[cfg(any(feature = "keyboard", feature = "music", feature = "workspaces"))]
pub struct IconLabel {
    container: gtk::Box,
    label: Label,
    image: Image,

    icon_theme: IconTheme,
    size: i32,
}

#[cfg(any(feature = "keyboard", feature = "music", feature = "workspaces"))]
impl IconLabel {
    pub fn new(input: &str, icon_theme: &IconTheme, size: i32) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let label = Label::builder().use_markup(true).build();
        label.add_class("icon");
        label.add_class("text-icon");

        let image = Image::new();
        image.add_class("icon");
        image.add_class("image");

        container.append(&image);
        container.append(&label);

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

    pub fn label(&self) -> &Label {
        &self.label
    }
}

impl Deref for IconLabel {
    type Target = gtk::Box;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}
