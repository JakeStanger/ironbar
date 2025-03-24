use gtk::Label;
use gtk::prelude::*;
use serde::Deserialize;

use super::{CustomWidget, CustomWidgetContext};
use crate::build;
use crate::config::{LayoutConfig, TruncateMode};
use crate::dynamic_value::dynamic_string;
use crate::gtk_helpers::IronbarLabelExt;

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

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    truncate: Option<TruncateMode>,
}

impl CustomWidget for LabelWidget {
    type Widget = Label;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let label = build!(self, Self::Widget);

        if !context.is_popup {
            // label.set_angle(self.layout.angle(context.info));
        }

        label.set_justify(self.layout.justify.into());
        label.set_use_markup(true);

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        {
            let label = label.clone();
            dynamic_string(&self.label, move |string| {
                label.set_label_escaped(&string);
            });
        }

        label
    }
}
