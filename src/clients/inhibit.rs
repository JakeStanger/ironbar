use crate::channels::SyncSenderExt;
use crate::register_client;
use gtk::ApplicationInhibitFlags;
use gtk::glib;
use gtk::prelude::*;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tracing::{error, trace};

fn get_app() -> gtk::Application {
    gtk::gio::Application::default()
        .and_downcast()
        .expect("GTK application not initialized")
}

/// Uninhibits on drop.
struct InhibitCookie(u32);

impl Drop for InhibitCookie {
    fn drop(&mut self) {
        trace!("dropped inhibit cookie: {}", self.0);
        get_app().uninhibit(self.0);
    }
}

fn gtk_inhibit() -> Option<InhibitCookie> {
    let app = get_app();
    let window = app.windows().into_iter().next();
    let id = app.inhibit(
        window.as_ref(),
        ApplicationInhibitFlags::IDLE,
        Some("Ironbar inhibit"),
    );
    if id == 0 {
        error!("GTK inhibit failed - platform may not support it");
        None
    } else {
        trace!("created inhibit cookie: {id}");
        Some(InhibitCookie(id))
    }
}

/// The held GTK cookie and its remaining duration: both exist, or neither.
#[derive(Default)]
struct Inhibitor {
    current: Option<(InhibitCookie, Duration)>,
}

impl Inhibitor {
    fn remaining(&self) -> Option<Duration> {
        self.current.as_ref().map(|(_, duration)| *duration)
    }

    fn is_counting_down(&self) -> bool {
        self.remaining()
            .is_some_and(|duration| duration != Duration::MAX)
    }

    /// `Some` starts the inhibit or updates its duration; `None` stops it.
    fn set_remaining(&mut self, target: Option<Duration>) {
        match target {
            None => self.current = None,
            Some(duration) => {
                if let Some((_, existing)) = &mut self.current {
                    *existing = duration;
                } else {
                    self.current = gtk_inhibit().map(|cookie| (cookie, duration));
                }
            }
        }
    }

    /// Decrements the countdown, dropping the cookie at zero.
    fn tick(&mut self) {
        if let Some((_, duration)) = &mut self.current {
            *duration = duration.saturating_sub(Duration::from_secs(1));
        }
        if self.remaining() == Some(Duration::ZERO) {
            self.current = None;
        }
    }
}

/// Process-global inhibit: owns the GTK cookie and the live countdown in a single
/// `glib` task. Each widget updates its label to the same remaining duration.
/// When the timer stops, each widget reverts to its currently-selected preset (`durations[idx]`).
#[derive(Debug)]
pub struct Client {
    req_tx: mpsc::UnboundedSender<Option<Duration>>,
    remaining_tx: watch::Sender<Option<Duration>>,
}

impl Client {
    /// Must be called on the GTK main thread - spawns the cookie/countdown task.
    pub(crate) fn new() -> Self {
        let (req_tx, mut rx) = mpsc::unbounded_channel::<Option<Duration>>();
        let (remaining_tx, _) = watch::channel(None::<Duration>);

        glib::spawn_future_local({
            let remaining_tx = remaining_tx.clone();
            async move {
                let mut inhibitor = Inhibitor::default();

                loop {
                    tokio::select! {
                        Some(request) = rx.recv() => inhibitor.set_remaining(request),
                        // Timer armed only while a finite countdown runs.
                        () = glib::timeout_future_seconds(1),
                            if inhibitor.is_counting_down() => inhibitor.tick(),
                    }

                    remaining_tx.send_replace(inhibitor.remaining());
                }
            }
        });

        Self {
            req_tx,
            remaining_tx,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<Duration>> {
        self.remaining_tx.subscribe()
    }

    /// Set the inhibit duration: `Some` starts it (or updates the duration if
    /// already inhibiting), `None` stops it. `Duration::MAX` = infinite.
    pub fn set_duration(&self, duration: Option<Duration>) {
        self.req_tx.send_expect(duration);
    }
}

register_client!(Client, inhibit);
