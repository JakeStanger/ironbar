use gtk::Image;
use gtk::prelude::*;
use serde::Deserialize;

use crate::build;
use crate::dynamic_value::dynamic_string;
use crate::image::ImageProvider;

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
    type Widget = Image;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let gtk_image = build!(self, Self::Widget);

        {
            let gtk_image = gtk_image.clone();
            let icon_theme = context.icon_theme.clone();

            dynamic_string(&self.src, move |src| {
                ImageProvider::parse(&src, &icon_theme, false, self.size)
                    .map(|image| image.load_into_image(&gtk_image));
            });
        }

        gtk_image
    }
}
