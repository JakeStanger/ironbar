use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;

use crate::build;
use crate::config::ModuleOrientation;
use crate::dynamic_value::dynamic_string;

use super::{CustomWidget, CustomWidgetContext};

#[derive(Debug, Deserialize, Clone)]
pub struct LabelWidget {
    name: Option<String>,
    class: Option<String>,
    label: String,
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
