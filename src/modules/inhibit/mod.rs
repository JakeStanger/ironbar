use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Button, Label};
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::inhibit;
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt, MouseButton};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};

mod config;

use config::InhibitCommand;
pub use config::InhibitModule;

const INFINITE_DURATION_LABEL: &str = "";

fn format_duration(d: Duration) -> String {
    if d == Duration::MAX {
        return INFINITE_DURATION_LABEL.to_string();
    }
    let s = d.as_secs();
    let (h, m, s) = (s / 3600, s % 3600 / 60, s % 60);
    match (h, m) {
        (h, m) if h > 0 => format!("{h:02}:{m:02}:{s:02}"),
        (_, m) => format!("{m:02}:{s:02}"),
    }
}

fn format_label(fmt: &str, duration: Duration) -> String {
    fmt.replace("{duration}", &format_duration(duration))
}

/// What a bar renders: the shared countdown when active, else this bar's preset.
#[derive(Debug, Clone, Copy)]
pub struct State {
    active: bool,
    duration: Duration,
}

impl Module<Button> for InhibitModule {
    type SendMessage = State;
    type ReceiveMessage = InhibitCommand;

    module_impl!("inhibit");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        ctx: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let client = ctx.client::<inhibit::Client>();
        let mut remaining_rx = client.subscribe();
        let tx = ctx.tx.clone();
        let durations = self.duration_spec.durations.clone();
        let default = self.duration_spec.default_duration;

        spawn(async move {
            let mut idx = durations.iter().position(|d| *d == default).unwrap_or(0);

            loop {
                let remaining = *remaining_rx.borrow();
                let state = State {
                    active: remaining.is_some(),
                    duration: remaining.unwrap_or(durations[idx]),
                };
                tx.send_update(state).await;

                tokio::select! {
                    changed = remaining_rx.changed() => if changed.is_err() { break },
                    cmd = rx.recv() => {
                        let Some(cmd) = cmd else { break }; // widget gone
                        match cmd {
                            InhibitCommand::Toggle if remaining.is_some() => client.set_duration(None),
                            InhibitCommand::Toggle => client.set_duration(Some(durations[idx])),
                            InhibitCommand::Cycle => {
                                idx = (idx + 1) % durations.len();
                                if remaining.is_some() {
                                    client.set_duration(Some(durations[idx]));
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        ctx: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<Button>> {
        let button = Button::new();
        button.add_css_class("inhibit");
        let label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .build();
        button.set_child(Some(&label));

        let tx = ctx.controller_tx.clone();
        [
            (MouseButton::Primary, self.on_click_left),
            (MouseButton::Secondary, self.on_click_right),
            (MouseButton::Middle, self.on_click_middle),
        ]
        .into_iter()
        .filter_map(|(btn, cmd)| cmd.map(|c| (btn, c)))
        .for_each(|(btn, cmd)| {
            let tx = tx.clone();
            button.connect_pressed(btn, move || tx.send_spawn(cmd));
        });

        let (fmt_on, fmt_off) = (self.format_on, self.format_off);
        ctx.subscribe().recv_glib(&label, move |label, state| {
            let fmt = if state.active { &fmt_on } else { &fmt_off };
            label.set_label_escaped(&format_label(fmt, state.duration));
        });

        Ok(ModuleParts::new(button, None))
    }
}
