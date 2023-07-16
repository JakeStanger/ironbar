use gtk::prelude::*;
use gtk::{Button, Label};
use serde::Deserialize;

use crate::dynamic_value::dynamic_string;
use crate::modules::PopupButton;
use crate::{build, try_send};

use super::{CustomWidget, CustomWidgetContext, ExecEvent};

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
        context.popup_buttons.borrow_mut().push(button.clone());

        if let Some(text) = self.label {
            let label = Label::new(None);
            label.set_use_markup(true);
            button.add(&label);

            dynamic_string(&text, move |string| {
                label.set_markup(&string);
                Continue(true)
            });
        }

        if let Some(exec) = self.on_click {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                try_send!(
                    tx,
                    ExecEvent {
                        cmd: exec.clone(),
                        args: None,
                        id: button.popup_id(),
                    }
                );
            });
        }

        button
    }
}
