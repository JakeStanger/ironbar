use gtk::ProgressBar;
use gtk::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::error;

use super::{CustomWidget, CustomWidgetContext};
use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::config::ModuleOrientation;
use crate::dynamic_value::DynamicString;
use crate::modules::custom::set_length;
use crate::script::{OutputStream, Script, ScriptInput};
use crate::{build, spawn};

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
#[serde(default)]
pub struct ProgressWidget {
    /// Widget name.
    name: Option<String>,

    /// Widget class name.
    class: Option<String>,

    /// Orientation of the progress bar.
    ///
    /// <br />
    #[serde(default)]
    orientation: ModuleOrientation,

    /// Text label to show for the progress bar
    label: Option<DynamicString>,

    /// Script to run to get the progress bar value.
    /// Output must be a valid percentage.
    ///
    /// Note that this expects a numeric value between `0`-`max` as output.
    value: Option<ScriptInput>,

    /// The maximum progress bar value.
    max: f64,

    /// The progress bar length, in pixels.
    /// GTK will automatically determine the size if left blank.
    length: Option<i32>,
}

impl Default for ProgressWidget {
    fn default() -> Self {
        Self {
            name: None,
            class: None,
            orientation: ModuleOrientation::default(),
            label: None,
            value: None,
            max: 100.0,
            length: None,
        }
    }
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

            rx.recv_glib((), move |(), value| progress.set_fraction(value / self.max));
        }

        if let Some(text) = self.label {
            progress.set_show_text(true);

            text.subscribe(&progress, move |progress, string| {
                progress.set_text(Some(&string));
            });
        }

        progress
    }
}
