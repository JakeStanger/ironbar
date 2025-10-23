use chrono::Utc;
use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Button, Label};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::{ProvidesClient, gtk_wayland};
use crate::gtk_helpers::{IronbarGtkExt, MouseButton};
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{module_impl, spawn};

mod config;

use config::InhibitCommand;
pub use config::InhibitModule;

const HOUR: u64 = 3600;

fn format_duration(duration: Duration) -> String {
    if duration == Duration::MAX {
        return "∞".to_string();
    }

    let secs = duration.as_secs();
    let (h, m, s) = (secs / HOUR, (secs % HOUR) / 60, secs % 60);

    match (h, m) {
        (h, m) if h > 0 => format!("{h:02}:{m:02}:{s:02}"),
        (_, m) if m > 0 => format!("{m:02}:{s:02}"),
        _ => format!("{s:02}s"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Inactive { selected_duration: Duration },
    Active { remaining: Duration },
}

fn get_state(client: &Arc<gtk_wayland::Client>, selected_duration: Duration) -> State {
    match client.inhibit_expiry() {
        None => State::Inactive { selected_duration },
        Some(expiry_time) if expiry_time == chrono::DateTime::<Utc>::MAX_UTC => State::Active {
            remaining: Duration::MAX,
        },
        Some(expiry_time) => match (expiry_time - Utc::now()).to_std().map(|d| d.as_secs()) {
            Ok(secs) if secs > 0 => State::Active {
                remaining: Duration::from_secs(secs),
            },
            _ => State::Inactive { selected_duration },
        },
    }
}

async fn handle_command(
    cmd: InhibitCommand,
    client: &Arc<gtk_wayland::Client>,
    durations: &[Duration],
    idx: &mut usize,
    tx: &impl AsyncSenderExt<ModuleUpdateEvent<State>>,
) -> State {
    let new_state = match cmd {
        InhibitCommand::Toggle => match get_state(client, durations[*idx]) {
            State::Active { .. } => {
                client.inhibit_stop();
                State::Inactive {
                    selected_duration: durations[*idx],
                }
            }
            State::Inactive { .. } => {
                client.inhibit_start(durations[*idx]);
                State::Active {
                    remaining: durations[*idx],
                }
            }
        },
        InhibitCommand::Cycle => {
            let was_active = matches!(get_state(client, durations[*idx]), State::Active { .. });
            *idx = (*idx + 1) % durations.len();
            if was_active {
                client.inhibit_start(durations[*idx]);
                State::Active {
                    remaining: durations[*idx],
                }
            } else {
                State::Inactive {
                    selected_duration: durations[*idx],
                }
            }
        }
    };
    tx.send_update(new_state).await;
    new_state
}

impl Module<Button> for InhibitModule {
    type SendMessage = State;
    type ReceiveMessage = InhibitCommand;

    module_impl!("inhibit");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        ctx: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = ctx.tx.clone();
        let duration_list = self.durations.clone();
        let default_duration = self.default_duration;
        let client: Arc<gtk_wayland::Client> = ctx.provide();

        spawn(async move {
            let mut idx = duration_list
                .iter()
                .position(|d| *d == default_duration)
                .unwrap_or(0);
            let mut state = get_state(&client, duration_list[idx]);
            tx.send_update(state).await;
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.tick().await;
            loop {
                tokio::select! {
                    Some(cmd) = rx.recv() => {
                        state = handle_command(cmd, &client, &duration_list, &mut idx, &tx).await;
                    }
                    _ = interval.tick() => {
                        let new_state = get_state(&client, duration_list[idx]);
                        if state != new_state {
                            state = new_state;
                            tx.send_update(new_state).await;
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
        .for_each(|(btn, action)| {
            let tx = tx.clone();
            button.connect_pressed(btn, move || {
                spawn({
                    let tx = tx.clone();
                    async move {
                        if let Err(e) = tx.send(action).await {
                            tracing::error!("Failed to send inhibit command: {e}");
                        }
                    }
                });
            });
        });
        let rx = ctx.subscribe();
        let format_on = self.format_on;
        let format_off = self.format_off;
        rx.recv_glib(&label, move |label, state| {
            let (format, duration) = match state {
                State::Active { remaining } => (&format_on, remaining),
                State::Inactive { selected_duration } => (&format_off, selected_duration),
            };
            let duration_str = format!("{:>7}", format_duration(duration));
            let text = format.replace("{duration}", &duration_str);
            label.set_label(&text);
        });
        Ok(ModuleParts::new(button, None))
    }
}
