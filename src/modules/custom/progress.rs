use gtk::prelude::*;
use gtk::ProgressBar;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::error;

use super::{CustomWidget, CustomWidgetContext};
use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::config::ModuleOrientation;
use crate::dynamic_value::dynamic_string;
use crate::modules::custom::set_length;
use crate::script::{OutputStream, Script, ScriptInput};
use crate::{build, spawn};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProgressWidget {
    /// Widget name.
    ///
    /// **Default**: `null`
    name: Option<String>,

    /// Widget class name.
    ///
    /// **Default**: `null`
    class: Option<String>,

    /// Orientation of the progress bar.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br />
    /// **Default**: `horizontal`
    #[serde(default)]
    orientation: ModuleOrientation,

    /// Text label to show for the progress bar.
    ///
    /// This is a [Dynamic String](dynamic-values#dynamic-string).
    ///
    /// **Default**: `null`
    label: Option<String>,

    /// Script to run to get the progress bar value.
    /// Output must be a valid percentage.
    ///
    /// Note that this expects a numeric value between `0`-`max` as output.
    ///
    /// **Default**: `null`
    value: Option<ScriptInput>,

    /// The maximum progress bar value.
    ///
    /// **Default**: `100`
    #[serde(default = "default_max")]
    max: f64,

    /// The progress bar length, in pixels.
    /// GTK will automatically determine the size if left blank.
    ///
    /// **Default**: `null`
    length: Option<i32>,
}

const fn default_max() -> f64 {
    100.0
}

impl CustomWidget for ProgressWidget {
    type Widget = ProgressBar;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let progress = build!(self, Self::Widget);

        progress.set_orientation(self.orientation.into());

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
                            Ok(value) => tx.send_spawn(value),
                            Err(err) => error!("{err:?}"),
                        },
                        OutputStream::Stderr(err) => error!("{err:?}"),
                    })
                    .await;
            });

            rx.recv_glib(move |value| progress.set_fraction(value / self.max));
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
