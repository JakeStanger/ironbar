use glib::Propagation;
use std::cell::Cell;
use std::ops::Neg;

use gtk::prelude::*;
use gtk::Scale;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::error;

use super::{CustomWidget, CustomWidgetContext, ExecEvent};
use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::config::ModuleOrientation;
use crate::modules::custom::set_length;
use crate::script::{OutputStream, Script, ScriptInput};
use crate::{build, spawn};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SliderWidget {
    /// Widget name.
    ///
    /// **Default**: `null`
    name: Option<String>,

    /// Widget class name.
    ///
    /// **Default**: `null`
    class: Option<String>,

    /// Orientation of the slider.
    ///
    /// **Valid options**: `horizontal`, `vertical`, `h`, `v`
    /// <br />
    /// **Default**: `horizontal`
    #[serde(default)]
    orientation: ModuleOrientation,

    /// Script to run to get the slider value.
    /// Output must be a valid number.
    ///
    /// **Default**: `null`
    value: Option<ScriptInput>,

    /// Command to execute when the slider changes.
    /// More on this [below](#slider).
    ///
    /// Note that this will provide the floating point value as an argument.
    /// If your input program requires an integer, you will need to round it.
    ///
    /// **Default**: `null`
    on_change: Option<String>,

    /// Minimum slider value.
    ///
    /// **Default**: `0`
    #[serde(default = "default_min")]
    min: f64,

    /// Maximum slider value.
    ///
    /// **Default**: `100`
    #[serde(default = "default_max")]
    max: f64,

    /// If the increment to change when scrolling with the mousewheel.
    /// If left blank, GTK will use the default value,
    /// determined by the current environment.
    ///
    /// **Default**: `null`
    step: Option<f64>,

    /// The slider length.
    /// GTK will automatically determine the size if left blank.
    ///
    /// **Default**: `null`
    length: Option<i32>,

    /// Whether to show the value label above the slider.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_label: bool,
}

const fn default_min() -> f64 {
    0.0
}

const fn default_max() -> f64 {
    100.0
}

impl CustomWidget for SliderWidget {
    type Widget = Scale;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget {
        let scale = build!(self, Self::Widget);

        scale.set_orientation(self.orientation.into());

        if let Some(length) = self.length {
            set_length(&scale, length, context.bar_orientation);
        }

        scale.set_range(self.min, self.max);
        scale.set_draw_value(self.show_label);

        if let Some(on_change) = self.on_change {
            let min = self.min;
            let max = self.max;
            let step = self.step;
            let tx = context.tx.clone();

            // GTK will spam the same value over and over
            let prev_value = Cell::new(scale.value());

            scale.connect_scroll_event(move |scale, event| {
                let value = scale.value();
                let delta = event.delta().1.neg();

                let delta = match (step, delta.is_sign_positive()) {
                    (Some(step), true) => step,
                    (Some(step), false) => -step,
                    (None, _) => delta,
                };

                scale.set_value(value + delta);
                Propagation::Proceed
            });

            scale.connect_change_value(move |_, _, val| {
                // GTK will send values outside min/max range
                let val = val.clamp(min, max);

                if val != prev_value.get() {
                    tx.send_spawn(ExecEvent {
                        cmd: on_change.clone(),
                        args: Some(vec![val.to_string()]),
                        id: usize::MAX, // ignored
                    });

                    prev_value.set(val);
                }

                Propagation::Proceed
            });
        }

        if let Some(value) = self.value {
            let script = Script::from(value);
            let scale = scale.clone();

            let (tx, rx) = mpsc::channel(128);

            spawn(async move {
                script
                    .run(None, move |stream, _success| match stream {
                        OutputStream::Stdout(out) => match out.parse() {
                            Ok(value) => tx.send_spawn(value),
                            Err(err) => error!("{err:?}"),
                        },
                        OutputStream::Stderr(err) => error!("{err:?}"),
                    })
                    .await;
            });

            rx.recv_glib(move |value| scale.set_value(value));
        }

        scale
    }
}
