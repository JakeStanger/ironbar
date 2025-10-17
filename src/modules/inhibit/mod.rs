use chrono::Utc;
use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Button, Label};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::gtk_helpers::{IronbarGtkExt, MouseButton};
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{module_impl, spawn};

mod config;
mod systemd;
mod wayland;

pub use config::{BackendType, InhibitCommand, InhibitModule};

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Inactive { selected_duration: Duration },
    Active { remaining: Duration },
}

enum Backend {
    Systemd(systemd::SystemdBackend),
    Wayland(wayland::WaylandBackend),
}

impl Backend {
    fn expiry(&self) -> Option<chrono::DateTime<Utc>> {
        match self {
            Backend::Systemd(b) => b.expiry,
            Backend::Wayland(b) => b.expiry,
        }
    }

    async fn start(&mut self, duration: Duration) -> Result<()> {
        match self {
            Backend::Systemd(b) => b.start(duration).await,
            Backend::Wayland(b) => b.start(duration).await,
        }
    }

    async fn stop(&mut self) -> Result<()> {
        match self {
            Backend::Systemd(b) => b.stop().await,
            Backend::Wayland(b) => b.stop().await,
        }
    }
}

fn calculate_expiry(duration: Duration) -> Option<chrono::DateTime<Utc>> {
    match duration {
        // Map Duration::MAX to DateTime::MAX for infinite inhibit
        Duration::MAX => Some(chrono::DateTime::<Utc>::MAX_UTC),
        d => Utc::now().checked_add_signed(chrono::Duration::from_std(d).ok()?),
    }
}

async fn create_backend(ty: BackendType) -> Result<Backend> {
    match ty {
        BackendType::Systemd => Ok(Backend::Systemd(systemd::SystemdBackend::new().await?)),
        BackendType::Wayland => Ok(Backend::Wayland(wayland::WaylandBackend::new().await?)),
    }
}

fn get_state(backend: &Backend, selected_duration: Duration) -> State {
    match backend.expiry() {
        None => State::Inactive { selected_duration },
        Some(dt) if dt == chrono::DateTime::<Utc>::MAX_UTC => State::Active {
            remaining: Duration::MAX,
        },
        Some(dt) => match (dt - Utc::now()).to_std().map(|d| d.as_secs()) {
            Ok(secs) if secs > 0 => State::Active {
                remaining: Duration::from_secs(secs),
            },
            _ => State::Inactive { selected_duration },
        },
    }
}

async fn handle_command(
    cmd: InhibitCommand,
    backend: &mut Backend,
    durations: &[Duration],
    idx: &mut usize,
    tx: &impl AsyncSenderExt<ModuleUpdateEvent<State>>,
) -> Result<State> {
    let current_state = get_state(backend, durations[*idx]);
    match (cmd, current_state) {
        (InhibitCommand::Toggle, State::Active { .. }) => {
            backend.stop().await.ok();
        }
        (InhibitCommand::Toggle, _) => {
            backend.start(durations[*idx]).await.ok();
        }
        (InhibitCommand::Cycle, current) => {
            *idx = (*idx + 1) % durations.len();
            if matches!(current, State::Active { .. }) {
                backend.start(durations[*idx]).await?;
            }
        }
    }
    let new_state = get_state(backend, durations[*idx]);
    tx.send_update(new_state.clone()).await;
    Ok(new_state)
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
        let (duration_list, default_index) = self.durations.clone();

        let backend_type = self.backend.expect("backend has default");
        spawn(async move {
            let mut backend = create_backend(backend_type)
                .await
                .expect("Failed to create inhibit backend");
            let mut idx = default_index;
            let mut state = get_state(&backend, duration_list[idx]);
            tx.send_update(state.clone()).await;
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.tick().await;
            loop {
                tokio::select! {
                    Some(cmd) = rx.recv() => {
                        if let Ok(new_state) = handle_command(cmd, &mut backend, &duration_list, &mut idx, &tx).await {
                            state = new_state;
                        }
                    }
                    _ = interval.tick() => {
                        let new_state = get_state(&backend, duration_list[idx]);
                        if matches!(new_state, State::Inactive { .. }) && !matches!(state, State::Inactive { .. }) {
                            backend.stop().await.ok();
                        }
                        if state != new_state {
                            state = new_state.clone();
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

        // Bind mouse buttons to actions
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
                let tx = tx.clone();
                spawn(async move {
                    tx.send(action).await.ok();
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
            let duration_str = match duration {
                Duration::MAX => format!("{:>7}", "∞"),
                d => format!("{:>7}", humantime::format_duration(d)),
            };
            let text = format.replace("{duration}", &duration_str);
            label.set_label(&text);
        });
        Ok(ModuleParts::new(button, None))
    }
}
