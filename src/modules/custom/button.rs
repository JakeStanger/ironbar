use gtk::prelude::*;
use gtk::{Button, Label, Orientation};
use serde::Deserialize;

use super::{CustomWidget, CustomWidgetContext, ExecEvent, WidgetConfig};
use crate::build;
use crate::channels::AsyncSenderExt;
use crate::config::ModuleOrientation;
use crate::dynamic_value::dynamic_string;
use crate::gtk_helpers::IronbarLabelExt;
use crate::modules::PopupButton;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ButtonWidget {
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
    /// This is a shorthand for adding a label widget to the button.
    /// Ignored if `widgets` is set.
    ///
    /// This is a [Dynamic String](dynamic-values#dynamic-string).
    ///
    /// **Default**: `null`
    label: Option<String>,

    /// Command to execute. More on this [below](#commands).
    ///
    /// **Default**: `null`
    on_click: Option<String>,

    /// Orientation of the button.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br />
    /// **Default**: `horizontal`
    #[serde(default)]
    orientation: ModuleOrientation,

    /// Modules and widgets to add to this box.
    ///
    /// **Default**: `null`
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

            label.set_angle(self.orientation.to_angle());

            button.add(&label);

            dynamic_string(&text, move |string| {
                label.set_label_escaped(&string);
            });
        }

        if let Some(exec) = self.on_click {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                tx.send_spawn(ExecEvent {
                    cmd: exec.clone(),
                    args: None,
                    id: button.try_popup_id().unwrap_or(usize::MAX), // may not be a popup button
                });
            });
        }

        button
    }
}
