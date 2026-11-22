use gtk::Label;
use gtk::prelude::*;
use serde::Deserialize;

use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::config::{LayoutConfig, TruncateMode};
use crate::dynamic_value::dynamic_string;
use crate::gtk_helpers::IronbarLabelExt;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
pub struct LabelWidget {
    /// Widget name.
    name: Option<String>,

    /// Widget class name.
    class: Option<String>,

    /// Widget text label. Pango markup and embedded scripts are supported.
    ///
    /// This is a [Dynamic String](dynamic-values#dynamic-string).
    label: String,

    // -- Common --

    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// The truncation options to use for the label text.
    truncate: Option<TruncateMode>,
}

impl CustomWidget for LabelWidget {
    type Widget = Label;

    fn into_widget(self, _context: CustomWidgetContext) -> Self::Widget {
        let label = build!(self, Self::Widget);

        label.set_justify(self.layout.justify.into());
        label.set_use_markup(true);

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        dynamic_string(&self.label, &label, move |label, string| {
            label.set_label_escaped(&string);
        });

        label
    }
}
