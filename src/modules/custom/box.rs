use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::config::ModuleOrientation;
use crate::modules::custom::WidgetConfig;
use gtk::{prelude::*, Orientation};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct BoxWidget {
    name: Option<String>,
    class: Option<String>,
    orientation: Option<ModuleOrientation>,
    widgets: Option<Vec<WidgetConfig>>,
}

impl CustomWidget for BoxWidget {
    type Widget = gtk::Box;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let container = build!(self, Self::Widget);

        if let Some(orientation) = self.orientation {
            container.set_orientation(
                Orientation::from(orientation),
            );
        }

        if let Some(widgets) = self.widgets {
            for widget in widgets {
                widget.widget.add_to(&container, &context, widget.common);
            }
        }

        container
    }
}
