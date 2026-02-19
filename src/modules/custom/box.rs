use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::config::ModuleOrientation;
use crate::modules::custom::WidgetConfig;
use gtk::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum ModuleAlignment {
    /// Align widget to the start (left for horizontal, top for vertical).
    Start,
    /// Align widget to the center.
    Center,
    /// Align widget to the end (right for horizontal, bottom for vertical).
    End,
    /// Stretch widget to fill available space.
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

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
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
    /// **Default**: `horizontal`
    orientation: Option<ModuleOrientation>,

    /// Horizontal alignment of the box relative to its parent.
    ///
    /// **Valid options**: `start`, `center`, `end`, `fill`
    /// **Default**: `fill`
    halign: Option<ModuleAlignment>,

    /// Vertical alignment of the box relative to its parent.
    ///
    /// **Valid options**: `start`, `center`, `end`, `fill`
    /// **Default**: `fill`
    valign: Option<ModuleAlignment>,

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

        let horizontal_alignment = self.halign.unwrap_or(ModuleAlignment::Fill);
        let vertical_alignment = self.valign.unwrap_or(ModuleAlignment::Fill);

        container.set_halign(horizontal_alignment.into());
        container.set_valign(vertical_alignment.into());

        container
    }
}
