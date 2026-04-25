use std::env;
use std::time::Duration;

use chrono::{DateTime, Local, Locale, Timelike};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Align, Button, Calendar, Label, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::{CommonConfig, LayoutConfig};
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{module_impl, spawn};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ClockModule {
    /// The format string to use for the date/time shown on the bar.
    /// Pango markup is supported.
    ///
    /// Detail on available tokens can be found here:
    /// <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    ///
    /// **Default**: `%d/%m/%Y %H:%M`
    format: String,

    /// The format string to use for the date/time shown in the popup header.
    /// Pango markup is supported.
    ///
    /// Detail on available tokens can be found here:
    /// <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    ///
    /// **Default**: `%H:%M:%S`
    format_popup: String,

    /// The locale to use when formatting dates.
    ///
    /// Note this will not control the calendar -
    /// for that you must set `LC_TIME`.
    ///
    /// **Valid options**: See [here](https://docs.rs/pure-rust-locales/0.8.1/pure_rust_locales/enum.Locale.html#variants)
    /// <br>
    /// **Default**: `$LC_TIME` or `$LANG` or `'POSIX'`
    locale: String,

    /// Whether to show the week numbers in the popup calendar
    ///
    /// **Default**: `false`
    show_week_numbers: bool,

    /// See [layout options](module-level-options#layout)
    #[serde(flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for ClockModule {
    fn default() -> Self {
        ClockModule {
            format: "%d/%m/%Y %H:%M".to_string(),
            format_popup: "%H:%M:%S".to_string(),
            locale: default_locale(),
            show_week_numbers: false,
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
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

/// Returns true if `format` (a chrono strftime template) contains any
/// specifier whose rendered value changes within a minute.
fn format_needs_second_precision(format: &str) -> bool {
    const MODIFIERS: &[u8] = b"-_0.";
    const SECOND_SPECIFIERS: &[u8] = b"STXrsc+f";

    let mut bytes = format.bytes();
    std::iter::from_fn(|| {
        bytes.find(|&b| b == b'%')?;
        bytes.find(|b| !b.is_ascii_digit() && !MODIFIERS.contains(b))
    })
    .any(|spec| SECOND_SPECIFIERS.contains(&spec))
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
        let needs_seconds = format_needs_second_precision(&self.format)
            || format_needs_second_precision(&self.format_popup);

        spawn(async move {
            loop {
                let date = Local::now();
                tx.send_update(date).await;
                // Sleep just past the next second/minute boundary
                let sub = u64::from(date.timestamp_subsec_millis());
                let to_next = if needs_seconds {
                    1000 - sub
                } else {
                    60 * 1000 - u64::from(date.second()) * 1000 - sub
                };
                // Overshoot boundary by 10ms
                sleep(Duration::from_millis(to_next + 10)).await;
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
            .use_markup(true)
            .justify(self.layout.justify.into())
            .build();

        button.set_child(Some(&label));

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let format = self.format.clone();
        let locale = Locale::try_from(self.locale.as_str()).unwrap_or(Locale::POSIX);

        let rx = context.subscribe();
        rx.recv_glib((), move |(), date| {
            let date_string = format!("{}", date.format_localized(&format, locale));
            label.set_label(&date_string);
        });

        let popup = self
            .into_popup(context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let container = gtk::Box::new(Orientation::Vertical, 0);

        let clock = Label::builder()
            .halign(Align::Center)
            .use_markup(true)
            .build();
        clock.add_css_class("calendar-clock");

        container.append(&clock);

        let calendar = Calendar::new();
        calendar.add_css_class("calendar");
        calendar.set_show_week_numbers(self.show_week_numbers);

        container.append(&calendar);

        let format = self.format_popup;
        let locale = Locale::try_from(self.locale.as_str()).unwrap_or(Locale::POSIX);

        context.subscribe().recv_glib((), move |(), date| {
            let date_string = format!("{}", date.format_localized(&format, locale));
            clock.set_label(&date_string);
        });

        // Reset selected date on each popup open
        context.popup.popover.connect_show(move |_| {
            let date = glib::DateTime::now_local().expect("should get current time");
            calendar.select_day(&date);
        });

        Some(container)
    }
}

#[cfg(test)]
mod tests {
    use super::format_needs_second_precision;

    #[test]
    fn format_needs_second_precision_classification() {
        let cases = [
            // Basic second-precision specifiers.
            ("%S", true),
            ("%T", true),
            ("%X", true),
            ("%r", true),
            ("%s", true),
            ("%c", true),
            ("%+", true),
            ("%f", true),
            ("%H:%M:%S", true),
            ("%Y-%m-%dT%H:%M:%S", true),
            // Modifier-prefixed forms must also be detected.
            ("%-S", true),
            ("%0S", true),
            ("%.f", true),
            ("%3f", true),
            ("%.6f", true),
            // Known limitation: this format actually make chrono _panic_, this test is simply here
            // to document the behaviour.
            ("%------S", true),
            // Minute-only / no-specifier formats.
            ("%H:%M", false),
            ("%R", false),
            ("%Y-%m-%d", false),
            ("%H", false),
            ("", false),
            ("plain text", false),
            // `%%` is a literal `%`, not the start of a specifier.
            ("%%S", false),
            ("%%-S", false),
            ("100%% load at %H:%M", false),
            ("%%S now %S", true),
            ("%dT%H:%M:%%S", false),
            // Dangling `%`
            ("%", false),
            ("ends with %", false),
            ("%-", false),
        ];
        for (fmt, expected) in cases {
            assert_eq!(
                format_needs_second_precision(fmt),
                expected,
                "format `{fmt}` expected needs_seconds={expected}"
            );
        }
    }
}
