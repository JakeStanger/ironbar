use crate::build;
use crate::dynamic_value::dynamic_string;
use gtk::prelude::*;
use gtk::{ContentFit, Picture};
use serde::Deserialize;

use super::{CustomWidget, CustomWidgetContext};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ImageWidget {
    /// Widget name.
    ///
    /// **Default**: `null`
    name: Option<String>,

    /// Widget class name.
    ///
    /// **Default**: `null`
    class: Option<String>,

    /// Image source.
    ///
    /// This is an [image](image) via [Dynamic String](dynamic-values#dynamic-string).
    ///
    /// **Required**
    src: String,

    /// The width/height of the image.
    /// Aspect ratio is preserved.
    ///
    /// **Default**: `32`
    #[serde(default = "default_size")]
    size: i32,
}

const fn default_size() -> i32 {
    32
}

impl CustomWidget for ImageWidget {
    type Widget = Picture;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let gtk_image = build!(self, Self::Widget);
        gtk_image.set_content_fit(ContentFit::ScaleDown);

        dynamic_string(&self.src, &gtk_image, move |gtk_image, src| {
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
