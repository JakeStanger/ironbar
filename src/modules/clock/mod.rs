mod popup;

use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use chrono::{DateTime, Local};
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::{Align, Button, Calendar, Label, Orientation};
use serde::Deserialize;
use tokio::spawn;
use tokio::sync::mpsc;
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
    type Message = DateTime<Local>;

    fn spawn_controller(
        &self,
        info: &ModuleInfo,
        tx: mpsc::Sender<ModuleUpdateEvent<Self::Message>>,
    ) -> Result<()> {
        spawn(async move {
            loop {
                let date = Local::now();
                tx.send(ModuleUpdateEvent::Update(date))
                    .await
                    .expect("Failed to send date");
                sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        Ok(())
    }

    fn into_widget(self, context: WidgetContext<Self::Message>) -> Result<ModuleWidget<Button>> {
        let button = Button::new();

        button.connect_clicked(move |button| {
            context
                .tx
                .try_send(super::ModuleUpdateEvent::TogglePopup)
                .expect("Failed to toggle popup");
        });

        let format = self.format.clone();
        {
            let button = button.clone();
            context.widget_rx.attach(None, move |date| {
                let date_string = format!("{}", date.format(&format));
                button.set_label(&date_string);
                Continue(true)
            });
        }

        let popup = self.into_popup(context.popup_rx)?;

        Ok(ModuleWidget {
            widget: button,
            popup: Some(popup),
        })
    }
}

impl ClockModule {
    fn into_popup(self, rx: glib::Receiver<DateTime<Local>>) -> Result<gtk::Box> {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .name("popup-clock")
            .build();

        let clock = Label::builder()
            .name("calendar-clock")
            .halign(Align::Center)
            .build();
        let format = "%H:%M:%S";

        container.add(&clock);

        let calendar = Calendar::builder().name("calendar").build();
        container.add(&calendar);

        {
            rx.attach(None, move |date| {
                let date_string = format!("{}", date.format(format));
                clock.set_label(&date_string);
                Continue(true)
            });
        }

        Ok(container)
    }
}
