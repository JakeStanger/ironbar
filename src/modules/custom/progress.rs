use gtk::prelude::*;
use gtk::Orientation;
use gtk::ProgressBar;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::error;

use crate::config::ModuleOrientation;
use crate::dynamic_value::dynamic_string;
use crate::modules::custom::set_length;
use crate::script::{OutputStream, Script, ScriptInput};
use crate::{build, glib_recv_mpsc, spawn, try_send};

use super::{CustomWidget, CustomWidgetContext};

#[derive(Debug, Deserialize, Clone)]
pub struct ProgressWidget {
    name: Option<String>,
    class: Option<String>,
    #[serde(default)]
    orientation: ModuleOrientation,
    label: Option<String>,
    value: Option<ScriptInput>,
    #[serde(default = "default_max")]
    max: f64,
    length: Option<i32>,
}

const fn default_max() -> f64 {
    100.0
}

impl CustomWidget for ProgressWidget {
    type Widget = ProgressBar;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let progress = build!(self, Self::Widget);

        progress.set_orientation(
            Orientation::from(self.orientation),
        );

        if let Some(length) = self.length {
            set_length(&progress, length, context.bar_orientation);
        }

        if let Some(value) = self.value {
            let script = Script::from(value);
            let progress = progress.clone();

            let (tx, rx) = mpsc::channel(128);

            spawn(async move {
                script
                    .run(None, move |stream, _success| match stream {
                        OutputStream::Stdout(out) => match out.parse::<f64>() {
                            Ok(value) => try_send!(tx, value),
                            Err(err) => error!("{err:?}"),
                        },
                        OutputStream::Stderr(err) => error!("{err:?}"),
                    })
                    .await;
            });

            glib_recv_mpsc!(rx, value => progress.set_fraction(value / self.max));
        }

        if let Some(text) = self.label {
            let progress = progress.clone();
            progress.set_show_text(true);

            dynamic_string(&text, move |string| {
                progress.set_text(Some(&string));
            });
        }

        progress
    }
}
