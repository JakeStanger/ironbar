pub use crate::popup::Popup;
use chrono::Local;
use gtk::prelude::*;
use gtk::{Align, Calendar, Label};
use tokio::spawn;
use tokio::time::sleep;

impl Popup {
    pub fn add_clock_widgets(&self) {
        let clock = Label::builder()
            .name("calendar-clock")
            .halign(Align::Center)
            .build();
        let format = "%H:%M:%S";

        self.container.add(&clock);

        let calendar = Calendar::builder().name("calendar").build();
        self.container.add(&calendar);

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        spawn(async move {
            loop {
                let date = Local::now();
                let date_string = format!("{}", date.format(format));

                tx.send(date_string).unwrap();
                sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        {
            rx.attach(None, move |s| {
                clock.set_label(s.as_str());
                Continue(true)
            });
        }
    }
}
