use std::env;

use chrono::{DateTime, Local, Locale};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Align, Button, Calendar, Label, Orientation};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};
use tokio::time::sleep;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::{CommonConfig, ModuleOrientation};
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{module_impl, spawn};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ClockModule {
    /// The format string to use for the date/time shown on the bar.
    /// Pango markup is supported.
    ///
    /// Detail on available tokens can be found here:
    /// <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    ///
    /// **Default**: `%d/%m/%Y %H:%M`
    #[serde(default = "default_format")]
    format: String,

    /// The format string to use for the date/time shown in the popup header.
    /// Pango markup is supported.
    ///
    /// Detail on available tokens can be found here:
    /// <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    ///
    /// **Default**: `%H:%M:%S`
    #[serde(default = "default_popup_format")]
    format_popup: String,

    /// The locale to use when formatting dates.
    ///
    /// Note this will not control the calendar -
    /// for that you must set `LC_TIME`.
    ///
    /// **Valid options**: See [here](https://docs.rs/pure-rust-locales/0.8.1/pure_rust_locales/enum.Locale.html#variants)
    /// <br>
    /// **Default**: `$LC_TIME` or `$LANG` or `'POSIX'`
    #[serde(default = "default_locale")]
    locale: String,

    /// The orientation to display the widget contents.
    /// Setting to vertical will rotate text 90 degrees.
    ///
    /// **Valid options**: `horizontal`, `vertical`
    /// <br>
    /// **Default**: `horizontal`
    #[serde(default)]
    orientation: ModuleOrientation,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for ClockModule {
    fn default() -> Self {
        ClockModule {
            format: default_format(),
            format_popup: default_popup_format(),
            locale: default_locale(),
            orientation: ModuleOrientation::Horizontal,
            common: Some(CommonConfig::default()),
        }
    }
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
        .map_or_else(|_| "POSIX".to_string(), strip_tail)
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

    module_impl!("clock");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        spawn(async move {
            loop {
                tx.send_update(Local::now()).await;
                sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<Button>> {
        let button = Button::new();
        let label = Label::builder()
            .angle(self.orientation.to_angle())
            .use_markup(true)
            .build();
        button.add(&label);

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let format = self.format.clone();
        let locale = Locale::try_from(self.locale.as_str()).unwrap_or(Locale::POSIX);

        context.subscribe().recv_glib(move |date| {
            let date_string = format!("{}", date.format_localized(&format, locale));
            label.set_label(&date_string);
        });

        let popup = self
            .into_popup(
                context.controller_tx.clone(),
                context.subscribe(),
                context,
                info,
            )
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        _tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let container = gtk::Box::new(Orientation::Vertical, 0);

        let clock = Label::builder()
            .halign(Align::Center)
            .use_markup(true)
            .build();
        clock.add_class("calendar-clock");

        container.add(&clock);

        let calendar = Calendar::new();
        calendar.add_class("calendar");
        container.add(&calendar);

        let format = self.format_popup;
        let locale = Locale::try_from(self.locale.as_str()).unwrap_or(Locale::POSIX);

        rx.recv_glib(move |date| {
            let date_string = format!("{}", date.format_localized(&format, locale));
            clock.set_label(&date_string);
        });

        container.show_all();

        Some(container)
    }
}
