use super::{CustomWidget, CustomWidgetContext};
use crate::dynamic_string::DynamicString;
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LabelWidget {
    name: Option<String>,
    class: Option<String>,
    label: Option<String>,
}

impl CustomWidget for LabelWidget {
    type Widget = Label;

    fn into_widget(self, _context: CustomWidgetContext) -> Self::Widget {
        let mut builder = Label::builder().use_markup(true);

        if let Some(name) = self.name {
            builder = builder.name(name);
        }

        let label = builder.build();

        if let Some(class) = self.class {
            label.style_context().add_class(&class);
        }

        let text = self.label.map_or_else(String::new, |text| text);

        {
            let label = label.clone();
            DynamicString::new(&text, move |string| {
                label.set_label(&string);
                Continue(true)
            });
        }

        label
    }
}
