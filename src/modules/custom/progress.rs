use super::{try_get_orientation, CustomWidget, CustomWidgetContext};
use crate::dynamic_string::DynamicString;
use crate::script::{OutputStream, Script, ScriptInput};
use crate::send;
use gtk::prelude::*;
use gtk::{Orientation, ProgressBar};
use serde::Deserialize;
use tokio::spawn;
use tracing::error;

#[derive(Debug, Deserialize, Clone)]
pub struct ProgressWidget {
    name: Option<String>,
    class: Option<String>,
    orientation: Option<String>,
    label: Option<String>,
    value: Option<ScriptInput>,
    #[serde(default = "default_max")]
    max: f64,
    length: Option<i32>,
}

const fn default_max() -> f64 {
    100.0
}

// TODO: Reduce duplication with slider, other widgets.
impl CustomWidget for ProgressWidget {
    type Widget = ProgressBar;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let mut builder = ProgressBar::builder();

        if let Some(name) = self.name {
            builder = builder.name(name);
        }

        if let Some(orientation) = self.orientation {
            builder = builder
                .orientation(try_get_orientation(&orientation).unwrap_or(context.bar_orientation));
        }

        if let Some(length) = self.length {
            builder = match context.bar_orientation {
                Orientation::Horizontal => builder.width_request(length),
                Orientation::Vertical => builder.height_request(length),
                _ => builder,
            }
        }

        let progress = builder.build();

        if let Some(class) = self.class {
            progress.style_context().add_class(&class);
        }

        if let Some(value) = self.value {
            let script = Script::from(value);
            let progress = progress.clone();

            let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

            spawn(async move {
                script
                    .run(None, move |stream, _success| match stream {
                        OutputStream::Stdout(out) => match out.parse::<f64>() {
                            Ok(value) => send!(tx, value),
                            Err(err) => error!("{err:?}"),
                        },
                        OutputStream::Stderr(err) => error!("{err:?}"),
                    })
                    .await;
            });

            rx.attach(None, move |value| {
                progress.set_fraction(value / self.max);
                Continue(true)
            });
        }

        if let Some(text) = self.label {
            let progress = progress.clone();
            progress.set_show_text(true);

            DynamicString::new(&text, move |string| {
                progress.set_text(Some(&string));
                Continue(true)
            });
        }

        progress
    }
}
