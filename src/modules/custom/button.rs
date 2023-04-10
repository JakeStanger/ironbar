use super::{CustomWidget, CustomWidgetContext, ExecEvent};
use crate::dynamic_string::DynamicString;
use crate::popup::Popup;
use crate::{build, try_send};
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
        let button = build!(self, Self::Widget);

        if let Some(text) = self.label {
            let label = Label::new(None);
            label.set_use_markup(true);
            button.add(&label);

            DynamicString::new(&text, move |string| {
                label.set_markup(&string);
                Continue(true)
            });
        }

        if let Some(exec) = self.on_click {
            let bar_orientation = context.bar_orientation;
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                try_send!(
                    tx,
                    ExecEvent {
                        cmd: exec.clone(),
                        args: None,
                        geometry: Popup::widget_geometry(button, bar_orientation),
                    }
                );
            });
        }

        button
    }
}
