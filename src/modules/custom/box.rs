use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::config::ModuleOrientation;
use crate::modules::custom::WidgetConfig;
use gtk::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
pub enum ModuleAlignment {
    /// Align widget to the start (left for horizontal, top for vertical).
    Start,
    /// Align widget to the center.
    Center,
    /// Align widget to the end (right for horizontal, bottom for vertical).
    End,
    /// Stretch widget to fill available space.
    #[default]
    Fill,
}

impl From<ModuleAlignment> for gtk::Align {
    fn from(align: ModuleAlignment) -> Self {
        match align {
            ModuleAlignment::Start => gtk::Align::Start,
            ModuleAlignment::Center => gtk::Align::Center,
            ModuleAlignment::End => gtk::Align::End,
            ModuleAlignment::Fill => gtk::Align::Fill,
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
#[serde(default)]
pub struct BoxWidget {
    /// Widget name.
    name: Option<String>,

    /// Widget class name.
    class: Option<String>,

    /// Whether child widgets should be horizontally or vertically added.
    orientation: Option<ModuleOrientation>,

    /// Horizontal alignment of the box relative to its parent.
    halign: ModuleAlignment,

    /// Vertical alignment of the box relative to its parent.
    valign: ModuleAlignment,

    /// Modules and widgets to add to this box.
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

        container.set_halign(self.halign.into());
        container.set_valign(self.valign.into());

        container
    }
}
