use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;

use crate::build;
use crate::config::ModuleOrientation;
use crate::dynamic_value::dynamic_string;

use super::{CustomWidget, CustomWidgetContext};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LabelWidget {
    /// Widget name.
    ///
    /// **Default**: `null`
    name: Option<String>,

    /// Widget class name.
    ///
    /// **Default**: `null`
    class: Option<String>,

    /// Widget text label. Pango markup and embedded scripts are supported.
    ///
    /// This is a [Dynamic String](dynamic-values#dynamic-string).
    ///
    /// **Required**
    label: String,

    /// Orientation of the label.
    /// Setting to vertical will rotate text 90 degrees.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br />
    /// **Default**: `horizontal`
    #[serde(default)]
    orientation: ModuleOrientation,
}

impl CustomWidget for LabelWidget {
    type Widget = Label;

    fn into_widget(self, _context: CustomWidgetContext) -> Self::Widget {
        let label = build!(self, Self::Widget);

        label.set_angle(self.orientation.to_angle());
        label.set_use_markup(true);

        {
            let label = label.clone();
            dynamic_string(&self.label, move |string| {
                label.set_markup(&string);
            });
        }

        label
    }
}
