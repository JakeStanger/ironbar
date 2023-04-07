use super::{try_get_orientation, CustomWidget, CustomWidgetContext, Widget};
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
        let mut builder = gtk::Box::builder();

        if let Some(name) = self.name {
            builder = builder.name(&name);
        }

        if let Some(orientation) = self.orientation {
            builder = builder
                .orientation(try_get_orientation(&orientation).unwrap_or(Orientation::Horizontal));
        }

        let container = builder.build();

        if let Some(class) = self.class {
            container.style_context().add_class(&class);
        }

        if let Some(widgets) = self.widgets {
            for widget in widgets {
                widget.add_to(&container, context);
            }
        }

        container
    }
}
