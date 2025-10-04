use crate::gtk_helpers::IronbarLabelExt;
use crate::image;
use gtk::prelude::*;
use gtk::{Button, ContentFit, Label, Orientation, Picture};
use std::ops::Deref;

#[derive(Debug, Clone)]
#[cfg(any(
    feature = "cairo",
    feature = "clipboard",
    feature = "clipboard",
    feature = "keyboard",
    feature = "launcher",
    feature = "music",
    feature = "notifications",
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
    feature = "notifications",
    feature = "workspaces",
))]
impl IconButton {
    pub fn new(input: &str, size: i32, image_provider: image::Provider) -> Self {
        let button = Button::new();
        let image = Picture::builder()
            .content_fit(ContentFit::ScaleDown)
            .build();
        let label = Label::builder().use_markup(true).build();
        label.set_label_escaped(input);

        if image::Provider::is_explicit_input(input) {
            image.add_css_class("image");
            image.add_css_class("icon");

            let image = image.clone();
            let label = label.clone();
            let button = button.clone();

            let input = input.to_string(); // ew

            glib::spawn_future_local(async move {
                if let Ok(true) = image_provider
                    .load_into_picture(&input, size, false, &image)
                    .await
                {
                    button.set_child(Some(&image));
                } else {
                    button.set_child(Some(&label));
                }
            });
        } else {
            button.set_child(Some(&label));
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
    feature = "notifications",
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

#[cfg(any(
    feature = "battery",
    feature = "bluetooth",
    feature = "keyboard",
    feature = "menu",
    feature = "music",
    feature = "workspaces",
))]
pub struct IconLabel {
    provider: image::Provider,
    container: gtk::Box,
    label: Label,
    image: Picture,

    size: i32,
}

#[cfg(any(
    feature = "battery",
    feature = "bluetooth",
    feature = "keyboard",
    feature = "menu",
    feature = "music",
    feature = "workspaces"
))]
impl IconLabel {
    pub fn new(input: &str, size: i32, image_provider: &image::Provider) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        let label = Label::builder().use_markup(true).build();
        label.add_css_class("icon");
        label.add_css_class("text-icon");

        let image = Picture::builder()
            .content_fit(ContentFit::ScaleDown)
            .build();
        image.add_css_class("icon");
        image.add_css_class("image");

        container.append(&image);
        container.append(&label);

        if image::Provider::is_explicit_input(input) {
            let image = image.clone();
            let label = label.clone();
            let image_provider = image_provider.clone();

            let input = input.to_string();

            glib::spawn_future_local(async move {
                let res = image_provider
                    .load_into_picture(&input, size, false, &image)
                    .await;
                if matches!(res, Ok(true)) {
                    image.set_visible(true);
                } else {
                    label.set_label_escaped(&input);
                    label.set_visible(true);
                }
            });
        } else {
            label.set_label_escaped(input);
            label.set_visible(true);
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
                    let res = provider
                        .load_into_picture(&input, size, false, &image)
                        .await;
                    if matches!(res, Ok(true)) {
                        label.set_visible(false);
                        image.set_visible(true);
                    } else {
                        label.set_label_escaped(&input);

                        image.set_visible(false);
                        label.set_visible(true);
                    }
                });
            } else {
                label.set_label_escaped(input);

                image.set_visible(false);
                label.set_visible(true);
            }
        } else {
            label.set_visible(false);
            image.set_visible(false);
        }
    }

    pub fn label(&self) -> &Label {
        &self.label
    }
}

#[cfg(any(
    feature = "battery",
    feature = "bluetooth",
    feature = "keyboard",
    feature = "menu",
    feature = "music",
    feature = "workspaces"
))]
impl Deref for IconLabel {
    type Target = gtk::Box;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

#[derive(Clone, Debug)]
#[cfg(feature = "music")]
pub struct IconPrefixedLabel {
    label: Label,
    container: gtk::Box,
}

#[cfg(feature = "music")]
impl IconPrefixedLabel {
    pub fn new(icon_input: &str, label: Option<&str>, image_provider: &image::Provider) -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 5);

        let icon = IconLabel::new(icon_input, 24, image_provider);

        let mut builder = Label::builder().use_markup(true);

        if let Some(label) = label {
            builder = builder.label(label);
        }

        let label = builder.build();

        icon.add_css_class("icon-box");
        label.add_css_class("label");

        container.append(&*icon);
        container.append(&label);

        Self { label, container }
    }

    pub fn label(&self) -> &Label {
        &self.label
    }
}

#[cfg(feature = "music")]
impl Deref for IconPrefixedLabel {
    type Target = gtk::Box;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}
