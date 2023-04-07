use super::{CustomWidget, CustomWidgetContext, ExecEvent};
use crate::popup::Popup;
use crate::try_send;
use gtk::prelude::*;
use gtk::{Button, Label};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ButtonWidget {
    name: Option<String>,
    class: Option<String>,
    label: Option<String>,
    on_click: Option<String>,
}

impl CustomWidget for ButtonWidget {
    type Widget = Button;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let mut builder = Button::builder();

        if let Some(name) = self.name {
            builder = builder.name(name);
        }

        let button = builder.build();

        if let Some(text) = self.label {
            let label = Label::new(None);
            label.set_use_markup(true);
            label.set_markup(&text);
            button.add(&label);
        }

        if let Some(class) = self.class {
            button.style_context().add_class(&class);
        }

        if let Some(exec) = self.on_click {
            let bar_orientation = context.bar_orientation;
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                try_send!(
                    tx,
                    ExecEvent {
                        cmd: exec.clone(),
                        geometry: Popup::button_pos(button, bar_orientation),
                    }
                );
            });
        }

        button
    }
}
