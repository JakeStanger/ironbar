mod popup;

use self::popup::Popup;
use crate::modules::{Module, ModuleInfo};
use chrono::Local;
use glib::Continue;
use gtk::prelude::*;
use gtk::{Button, Orientation};
use serde::Deserialize;
use tokio::spawn;
use tokio::time::sleep;

#[derive(Debug, Deserialize, Clone)]
pub struct ClockModule {
    /// Date/time format string.
    /// Default: `%d/%m/%Y %H:%M`
    ///
    /// Detail on available tokens can be found here:
    /// <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    #[serde(default = "default_format")]
    pub(crate) format: String,
}

fn default_format() -> String {
    String::from("%d/%m/%Y %H:%M")
}

impl Module<Button> for ClockModule {
    fn into_widget(self, info: &ModuleInfo) -> Button {
        let button = Button::new();

        let popup = Popup::new(
            "popup-clock",
            info.app,
            info.monitor,
            Orientation::Vertical,
            info.bar_position,
        );
        popup.add_clock_widgets();

        button.show_all();

        button.connect_clicked(move |button| {
            popup.show(button);
        });

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            let format = self.format.as_str();
            loop {
                let date = Local::now();
                let date_string = format!("{}", date.format(format));

                tx.send(date_string).unwrap();
                sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        {
            let button = button.clone();
            rx.attach(None, move |s| {
                button.set_label(s.as_str());
                Continue(true)
            });
        }

        button
    }
}
