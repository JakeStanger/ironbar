use super::{try_get_orientation, CustomWidget, CustomWidgetContext, ExecEvent};
use crate::modules::custom::set_length;
use crate::popup::Popup;
use crate::script::{OutputStream, Script, ScriptInput};
use crate::{build, send, try_send};
use gtk::prelude::*;
use gtk::Scale;
use serde::Deserialize;
use std::cell::Cell;
use tokio::spawn;
use tracing::error;

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
    length: Option<i32>,
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

        if let Some(on_change) = self.on_change {
            let min = self.min;
            let max = self.max;
            let tx = context.tx.clone();

            // GTK will spam the same value over and over
            let prev_value = Cell::new(scale.value());

            scale.connect_change_value(move |scale, _, val| {
                // GTK will send values outside min/max range
                let val = val.clamp(min, max);

                if val != prev_value.get() {
                    try_send!(
                        tx,
                        ExecEvent {
                            cmd: on_change.clone(),
                            args: Some(vec![val.to_string()]),
                            geometry: Popup::widget_geometry(scale, context.bar_orientation),
                        }
                    );

                    prev_value.set(val);
                }

                Inhibit(false)
            });
        }

        if let Some(value) = self.value {
            let script = Script::from(value);
            let scale = scale.clone();

            let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

            spawn(async move {
                script
                    .run(None, move |stream, _success| match stream {
                        OutputStream::Stdout(out) => match out.parse() {
                            Ok(value) => send!(tx, value),
                            Err(err) => error!("{err:?}"),
                        },
                        OutputStream::Stderr(err) => error!("{err:?}"),
                    })
                    .await;
            });

            rx.attach(None, move |value| {
                scale.set_value(value);
                Continue(true)
            });
        }

        scale
    }
}
