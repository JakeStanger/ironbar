use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Button, Label};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tracing::{debug, trace};

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::inhibit;
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt, MouseButton};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};

mod config;

use config::InhibitCommand;
pub use config::InhibitModule;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    active: bool,
    duration: Duration,
}

fn format_duration(d: Duration) -> String {
    if d == Duration::MAX {
        return "".to_string();
    }
    let s = d.as_secs();
    let (h, m, s) = (s / 3600, s % 3600 / 60, s % 60);
    match (h, m) {
        (h, m) if h > 0 => format!("{h:02}:{m:02}:{s:02}"),
        (_, m) => format!("{m:02}:{s:02}"),
    }
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
        let tx = ctx.tx.clone();
        let durations = self.duration_spec.durations.clone();
        let default = self.duration_spec.default_duration;

        spawn(async move {
            debug!("Inhibit controller started");
            let mut idx = durations.iter().position(|d| *d == default).unwrap_or(0);
            let mut state = State {
                active: false,
                duration: durations[idx],
            };
            tx.send_update(state).await;

            loop {
                tokio::select! {
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            InhibitCommand::Cycle => idx = (idx + 1) % durations.len(),
                            InhibitCommand::Toggle => state.active = !state.active,
                        }
                        state.duration = durations[idx];
                        trace!("Inhibit state update: active={}, duration={}", state.active, format_duration(state.duration));
                        tx.send_update(state).await;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(1)), if state.active && state.duration != Duration::MAX => {
                        state.duration = state.duration.saturating_sub(Duration::from_secs(1));
                        if state.duration == Duration::ZERO {
                            state.active = false;
                            state.duration = durations[idx];
                        }
                        tx.send_update(state).await;
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
        .for_each(|(btn, action)| {
            let tx = tx.clone();
            button.connect_pressed(btn, move || {
                tx.send_spawn(action);
            });
        });

        let client = ctx.client::<inhibit::Client>();
        let inhibit_handle = std::cell::RefCell::new(None::<Arc<inhibit::InhibitCookie>>);
        let was_active = std::cell::Cell::new(false);
        let (fmt_on, fmt_off) = (self.format_on, self.format_off);
        let controller_tx = ctx.controller_tx.clone();

        // gtk based inhibit() requires glib context / main thread
        ctx.subscribe().recv_glib(&label, move |label, state| {
            if state.active != was_active.replace(state.active) {
                if state.active {
                    match client.acquire() {
                        Some(cookie) => *inhibit_handle.borrow_mut() = Some(cookie),
                        None => {
                            controller_tx.send_spawn(InhibitCommand::Toggle);
                            return;
                        }
                    }
                } else {
                    *inhibit_handle.borrow_mut() = None;
                }
            }

            let fmt = if state.active { &fmt_on } else { &fmt_off };
            label.set_label_escaped(&fmt.replace("{duration}", &format_duration(state.duration)));
        });
        Ok(ModuleParts::new(button, None))
    }
}
