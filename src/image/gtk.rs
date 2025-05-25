use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::image;
use gtk::prelude::*;
use gtk::{Button, Image, Label, Orientation};
use std::ops::Deref;

#[derive(Debug, Clone)]
#[cfg(any(
    feature = "cairo",
    feature = "clipboard",
    feature = "clipboard",
    feature = "keyboard",
    feature = "launcher",
    feature = "music",
    feature = "workspaces",
))]
pub struct IconButton {
    button: Button,
    label: Label,
}

#[cfg(any(
    feature = "cairo",
    feature = "clipboard",
    feature = "clipboard",
    feature = "keyboard",
    feature = "launcher",
    feature = "music",
    feature = "workspaces",
))]
impl IconButton {
    pub fn new(input: &str, size: i32, image_provider: image::Provider) -> Self {
        let button = Button::new();
        let image = Image::new();
        let label = Label::new(Some(input));

        if image::Provider::is_explicit_input(input) {
            image.add_class("image");
            image.add_class("icon");

            let image = image.clone();
            let label = label.clone();
            let button = button.clone();

            let input = input.to_string(); // ew

            glib::spawn_future_local(async move {
                if let Ok(true) = image_provider
                    .load_into_image(&input, size, false, &image)
                    .await
                {
                    button.set_image(Some(&image));
                    button.set_always_show_image(true);
                } else {
                    button.set_child(Some(&label));
                    label.show();
                }
            });
        } else {
            button.set_child(Some(&label));
            label.show();
        }

        Self { button, label }
    }

    pub fn label(&self) -> &Label {
        &self.label
    }
}

#[cfg(any(
    feature = "clipboard",
    feature = "keyboard",
    feature = "music",
    feature = "workspaces",
    feature = "cairo",
    feature = "clipboard",
    feature = "launcher",
))]
impl Deref for IconButton {
    type Target = Button;

    fn deref(&self) -> &Self::Target {
        &self.button
    }
}

#[cfg(any(feature = "keyboard", feature = "music", feature = "workspaces"))]
pub struct IconLabel {
    provider: image::Provider,
    container: gtk::Box,
    label: Label,
    image: Image,

    size: i32,
}

#[cfg(any(feature = "keyboard", feature = "music", feature = "workspaces"))]
impl IconLabel {
    pub fn new(input: &str, size: i32, image_provider: &image::Provider) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let label = Label::builder().use_markup(true).build();
        label.add_class("icon");
        label.add_class("text-icon");

        let image = Image::new();
        image.add_class("icon");
        image.add_class("image");

        container.add(&image);
        container.add(&label);

        if image::Provider::is_explicit_input(input) {
            let image = image.clone();
            let label = label.clone();
            let image_provider = image_provider.clone();

            let input = input.to_string();

            glib::spawn_future_local(async move {
                let res = image_provider
                    .load_into_image(&input, size, false, &image)
                    .await;
                if matches!(res, Ok(true)) {
                    image.show();
                } else {
                    label.set_text(&input);
                    label.show();
                }
            });
        } else {
            label.set_text(input);
            label.show();
        }

        Self {
            provider: image_provider.clone(),
            container,
            label,
            image,
            size,
        }
    }

    pub fn set_label(&self, input: Option<&str>) {
        let label = &self.label;
        let image = &self.image;

        if let Some(input) = input {
            if image::Provider::is_explicit_input(input) {
                let provider = self.provider.clone();
                let size = self.size;

                let label = label.clone();
                let image = image.clone();
                let input = input.to_string();

                glib::spawn_future_local(async move {
                    let res = provider.load_into_image(&input, size, false, &image).await;
                    if matches!(res, Ok(true)) {
                        label.hide();
                        image.show();
                    } else {
                        label.set_label_escaped(&input);

                        image.hide();
                        label.show();
                    }
                });
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

#[cfg(any(feature = "keyboard", feature = "music", feature = "workspaces"))]
impl Deref for IconLabel {
    type Target = gtk::Box;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}
