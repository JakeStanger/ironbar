use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::config::ModuleOrientation;
use crate::modules::custom::WidgetConfig;
use gtk::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct BoxWidget {
    /// Widget name.
    ///
    /// **Default**: `null`
    name: Option<String>,

    /// Widget class name.
    ///
    /// **Default**: `null`
    class: Option<String>,

    /// Whether child widgets should be horizontally or vertically added.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br />
    /// **Default**: `horizontal`
    orientation: Option<ModuleOrientation>,

    /// Modules and widgets to add to this box.
    ///
    /// **Default**: `null`
    widgets: Option<Vec<WidgetConfig>>,
}

impl CustomWidget for BoxWidget {
    type Widget = gtk::Box;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let container = build!(self, Self::Widget);

        if let Some(orientation) = self.orientation {
            container.set_orientation(orientation.into());
        }

        if let Some(widgets) = self.widgets {
            for widget in widgets {
                widget.widget.add_to(&container, &context, widget.common);
            }
        }

        container
    }
}
