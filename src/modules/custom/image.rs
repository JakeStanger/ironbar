use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::image::ImageProvider;
use gtk::prelude::*;
use gtk::Image;
use serde::Deserialize;
use tracing::error;

#[derive(Debug, Deserialize, Clone)]
pub struct ImageWidget {
    name: Option<String>,
    class: Option<String>,
    src: Option<String>,
    size: Option<i32>,
}

impl CustomWidget for ImageWidget {
    type Widget = Image;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let gtk_image = build!(self, Self::Widget);

        if let Some(src) = self.src {
            let size = self.size.unwrap_or(32);
            if let Err(err) = ImageProvider::parse(&src, context.icon_theme, size)
                .and_then(|image| image.load_into_image(gtk_image.clone()))
            {
                error!("{err:?}");
            }
        }

        gtk_image
    }
}
