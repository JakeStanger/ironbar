use gtk::prelude::*;
use gtk::{Button, Label, Orientation};
use serde::Deserialize;

use crate::dynamic_value::dynamic_string;
use crate::modules::PopupButton;
use crate::{build, try_send};

use super::{CustomWidget, CustomWidgetContext, ExecEvent, WidgetConfig};

#[derive(Debug, Deserialize, Clone)]
pub struct ButtonWidget {
    name: Option<String>,
    class: Option<String>,
    label: Option<String>,
    on_click: Option<String>,
    widgets: Option<Vec<WidgetConfig>>,
}

impl CustomWidget for ButtonWidget {
    type Widget = Button;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let button = build!(self, Self::Widget);
        context.popup_buttons.borrow_mut().push(button.clone());

        if let Some(widgets) = self.widgets {
            let container = gtk::Box::new(Orientation::Horizontal, 0);

            for widget in widgets {
                widget.widget.add_to(&container, &context, widget.common);
            }

            button.add(&container);
        } else if let Some(text) = self.label {
            let label = Label::new(None);
            label.set_use_markup(true);
            button.add(&label);

            dynamic_string(&text, move |string| {
                label.set_markup(&string);
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
                        id: button.try_popup_id().unwrap_or(usize::MAX), // may not be a popup button
                    }
                );
            });
        }

        button
    }
}
