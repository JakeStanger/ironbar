use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::dynamic_string::DynamicString;
use crate::image::ImageProvider;
use gtk::prelude::*;
use gtk::Image;
use serde::Deserialize;
use tracing::error;

#[derive(Debug, Deserialize, Clone)]
pub struct ImageWidget {
    name: Option<String>,
    class: Option<String>,
    src: String,
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

            DynamicString::new(&self.src, move |src| {
                let res = ImageProvider::parse(&src, &icon_theme, self.size)
                    .and_then(|image| image.load_into_image(gtk_image.clone()));

                if let Err(err) = res {
                    error!("{err:?}");
                }

                Continue(true)
            });
        }

        gtk_image
    }
}
