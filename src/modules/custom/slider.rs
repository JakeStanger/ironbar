use glib::Propagation;
use std::cell::Cell;
use std::ops::Neg;

use gtk::prelude::*;
use gtk::Scale;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::error;

use crate::modules::custom::set_length;
use crate::script::{OutputStream, Script, ScriptInput};
use crate::{build, glib_recv_mpsc, spawn, try_send};

use super::{try_get_orientation, CustomWidget, CustomWidgetContext, ExecEvent};

#[derive(Debug, Deserialize, Clone)]
pub struct SliderWidget {
    name: Option<String>,
    class: Option<String>,
    orientation: Option<String>,
    value: Option<ScriptInput>,
    on_change: Option<String>,
    #[serde(default = "default_min")]
    min: f64,
    #[serde(default = "default_max")]
    max: f64,
    step: Option<f64>,
    length: Option<i32>,
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

        if let Some(orientation) = self.orientation {
            scale.set_orientation(
                try_get_orientation(&orientation).unwrap_or(context.bar_orientation),
            );
        }

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
                    try_send!(
                        tx,
                        ExecEvent {
                            cmd: on_change.clone(),
                            args: Some(vec![val.to_string()]),
                            id: usize::MAX // ignored
                        }
                    );

                    prev_value.set(val);
                }

                Propagation::Proceed
            });
        }

        if let Some(value) = self.value {
            let script = Script::from(value);
            let scale = scale.clone();

            let (tx, mut rx) = mpsc::channel(128);

            spawn(async move {
                script
                    .run(None, move |stream, _success| match stream {
                        OutputStream::Stdout(out) => match out.parse() {
                            Ok(value) => try_send!(tx, value),
                            Err(err) => error!("{err:?}"),
                        },
                        OutputStream::Stderr(err) => error!("{err:?}"),
                    })
                    .await;
            });

            glib_recv_mpsc!(rx, value => scale.set_value(value));
        }

        scale
    }
}
