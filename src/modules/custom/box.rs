use super::{try_get_orientation, CustomWidget, CustomWidgetContext, Widget};
use crate::build;
use gtk::prelude::*;
use gtk::Orientation;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct BoxWidget {
    name: Option<String>,
    class: Option<String>,
    orientation: Option<String>,
    widgets: Option<Vec<Widget>>,
}

impl CustomWidget for BoxWidget {
    type Widget = gtk::Box;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let container = build!(self, Self::Widget);

        if let Some(orientation) = self.orientation {
            container.set_orientation(
                try_get_orientation(&orientation).unwrap_or(Orientation::Horizontal),
            );
        }

        if let Some(widgets) = self.widgets {
            for widget in widgets {
                widget.add_to(&container, context);
            }
        }

        container
    }
}
