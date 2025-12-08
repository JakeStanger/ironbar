use crate::gtk_helpers::IronbarLabelExt;
use crate::image;
use gtk::prelude::*;
use gtk::{Button, ContentFit, Image, Label, Orientation, Picture};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

const ICON_PREFIX: &str = "icon:";
const IMAGE_CLASSES: &[&str] = &["icon", "image"];

fn create_icon<F>(input: &str, size: i32, provider: &image::Provider, on_result: F)
where
    F: FnOnce(Result<gtk::Widget, ()>) + 'static,
{
    // Uses Image for themed icons (icon: prefix) or Picture for files (file://, http://).
    if let Some(icon_name) = input.strip_prefix(ICON_PREFIX) {
        let image = Image::builder().build();
        image.set_css_classes(IMAGE_CLASSES);
        image.set_paintable(Some(&provider.lookup_icon(
            icon_name,
            size,
            image.scale_factor(),
        )));
        image.set_pixel_size(size);
        on_result(Ok(image.upcast()));
    } else {
        let picture = Picture::builder()
            .content_fit(ContentFit::ScaleDown)
            .build();
        picture.set_css_classes(IMAGE_CLASSES);

        let provider = provider.clone();
        let input = input.to_owned();
        let picture_clone = picture.clone();

        glib::spawn_future_local(async move {
            if provider
                .load_into_picture(&input, size, false, &picture_clone)
                .await
                .unwrap_or(false)
            {
                on_result(Ok(picture_clone.upcast()));
            } else {
                on_result(Err(()));
            }
        });
    }
}

#[derive(Debug, Clone)]
#[cfg(any(
    feature = "cairo",
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
    feature = "keyboard",
    feature = "launcher",
    feature = "music",
    feature = "notifications",
    feature = "workspaces",
))]
impl IconButton {
    pub fn new(input: &str, size: i32, image_provider: image::Provider) -> Self {
        let button = Button::new();
        let label = Label::builder().use_markup(true).build();
        label.set_label_escaped(input);

        if image::Provider::is_explicit_input(input) {
            let button_clone = button.clone();
            let label_clone = label.clone();

            create_icon(input, size, &image_provider, move |result| match result {
                Ok(widget) => button_clone.set_child(Some(&widget)),
                Err(()) => button_clone.set_child(Some(&label_clone)),
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
    feature = "cairo",
    feature = "clipboard",
    feature = "keyboard",
    feature = "launcher",
    feature = "music",
    feature = "notifications",
    feature = "workspaces",
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
    current_icon: Rc<RefCell<Option<gtk::Widget>>>,

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

        let current_icon = Rc::new(RefCell::new(None));

        if image::Provider::is_explicit_input(input) {
            let label_clone = label.clone();
            let input_str = input.to_owned();
            let container_clone = container.clone();
            let current_icon_clone = current_icon.clone();

            create_icon(input, size, image_provider, move |result| match result {
                Ok(widget) => {
                    // This executes after the label is appended below, so prepend is used to keep the order.
                    container_clone.prepend(&widget);
                    *current_icon_clone.borrow_mut() = Some(widget);
                }
                Err(()) => {
                    label_clone.set_label_escaped(&input_str);
                    label_clone.set_visible(true);
                }
            });
        } else {
            label.set_label_escaped(input);
            label.set_visible(true);
        }

        container.append(&label);

        Self {
            provider: image_provider.clone(),
            container,
            label,
            current_icon,
            size,
        }
    }

    pub fn set_label(&self, input: Option<&str>) {
        // Remove old icon if present
        if let Some(old) = self.current_icon.borrow_mut().take() {
            self.container.remove(&old);
        }

        match input {
            Some(input) if image::Provider::is_explicit_input(input) => {
                self.label.set_visible(false);
                let label_clone = self.label.clone();
                let input_str = input.to_owned();
                let container_clone = self.container.clone();
                let current_icon_clone = self.current_icon.clone();

                create_icon(
                    input,
                    self.size,
                    &self.provider,
                    move |result| match result {
                        Ok(widget) => {
                            container_clone.prepend(&widget);
                            *current_icon_clone.borrow_mut() = Some(widget);
                        }
                        Err(()) => {
                            label_clone.set_label_escaped(&input_str);
                            label_clone.set_visible(true);
                        }
                    },
                );
            }
            Some(input) => {
                self.label.set_label_escaped(input);
                self.label.set_visible(true);
            }
            None => {
                self.label.set_visible(false);
            }
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
