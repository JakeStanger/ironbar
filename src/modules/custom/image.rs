use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::config::default;
use crate::dynamic_value::DynamicString;
use gtk::prelude::*;
use gtk::{ContentFit, Picture};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
#[serde(default)]
pub struct ImageWidget {
    /// Widget name.
    name: Option<String>,

    /// Widget class name.
    class: Option<String>,

    /// Image source.
    ///
    /// This is an [image](/guides/images).
    src: DynamicString,

    /// The width/height of the image.
    /// Aspect ratio is preserved.
    size: i32,
}

impl Default for ImageWidget {
    fn default() -> Self {
        Self {
            name: None,
            class: None,
            src: DynamicString::default(),
            size: default::IconSize::Normal as i32,
        }
    }
}

impl CustomWidget for ImageWidget {
    type Widget = Picture;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let gtk_image = build!(self, Self::Widget);
        gtk_image.set_content_fit(ContentFit::ScaleDown);

        self.src.subscribe(&gtk_image, move |gtk_image, src| {
            let gtk_image = gtk_image.clone();
            let image_provider = context.image_provider.clone();
            glib::spawn_future_local(async move {
                image_provider
                    .load_into_picture_silent(&src, self.size, false, &gtk_image)
                    .await;
            });
        });

        gtk_image
    }
}
