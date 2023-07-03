use crate::config::CommonConfig;
use crate::gtk_helpers::add_class;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::Popup;
use crate::{send_async, try_send};
use chrono::{DateTime, Local, Locale};
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::{Align, Button, Calendar, Label, Orientation};
use serde::Deserialize;
use std::env;
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
    format: String,

    #[serde(default = "default_popup_format")]
    format_popup: String,

    #[serde(default = "default_locale")]
    locale: String,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_format() -> String {
    String::from("%d/%m/%Y %H:%M")
}

fn default_popup_format() -> String {
    String::from("%H:%M:%S")
}

fn default_locale() -> String {
    env::var("LC_TIME")
        .or_else(|_| env::var("LANG"))
        .map(strip_tail)
        .unwrap_or_else(|_| "POSIX".to_string())
}

fn strip_tail(string: String) -> String {
    string
        .split_once('.')
        .map(|(head, _)| head.to_string())
        .unwrap_or(string)
}

impl Module<Button> for ClockModule {
    type SendMessage = DateTime<Local>;
    type ReceiveMessage = ();

    fn name() -> &'static str {
        "clock"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: mpsc::Sender<ModuleUpdateEvent<Self::SendMessage>>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        spawn(async move {
            loop {
                let date = Local::now();
                send_async!(tx, ModuleUpdateEvent::Update(date));
                sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<Button>> {
        let button = Button::new();
        let label = Label::new(None);
        label.set_angle(info.bar_position.get_angle());
        button.add(&label);

        let orientation = info.bar_position.get_orientation();
        button.connect_clicked(move |button| {
            try_send!(
                context.tx,
                ModuleUpdateEvent::TogglePopup(Popup::widget_geometry(button, orientation))
            );
        });

        let format = self.format.clone();
        let locale = Locale::try_from(self.locale.as_str()).unwrap_or(Locale::POSIX);

        context.widget_rx.attach(None, move |date| {
            let date_string = format!("{}", date.format_localized(&format, locale));
            label.set_label(&date_string);
            Continue(true)
        });

        let popup = self.into_popup(context.controller_tx, context.popup_rx, info);

        Ok(ModuleWidget {
            widget: button,
            popup,
        })
    }

    fn into_popup(
        self,
        _tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: glib::Receiver<Self::SendMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let container = gtk::Box::new(Orientation::Vertical, 0);

        let clock = Label::builder().halign(Align::Center).build();
        add_class(&clock, "calendar-clock");

        container.add(&clock);

        let calendar = Calendar::new();
        add_class(&calendar, "calendar");
        container.add(&calendar);

        let format = self.format_popup;
        let locale = Locale::try_from(self.locale.as_str()).unwrap_or(Locale::POSIX);

        rx.attach(None, move |date| {
            let date_string = format!("{}", date.format_localized(&format, locale));
            clock.set_label(&date_string);
            Continue(true)
        });

        container.show_all();

        Some(container)
    }
}
