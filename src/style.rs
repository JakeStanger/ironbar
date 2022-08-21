use color_eyre::{Help, Report};
use glib::Continue;
use gtk::prelude::CssProviderExt;
use gtk::{gdk, gio, CssProvider, StyleContext};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tokio::spawn;
use tracing::{error, info};

pub fn load_css(style_path: PathBuf) {
    let provider = CssProvider::new();

    if let Err(err) = provider.load_from_file(&gio::File::for_path(&style_path)) {
        error!("{:?}", Report::new(err)
                    .wrap_err("Failed to load CSS")
                    .suggestion("Check the CSS file for errors")
                    .suggestion("GTK CSS uses a subset of the full CSS spec and many properties are not available. Ensure you are not using any unsupported property.")
                );
    }

    let screen = gdk::Screen::default().expect("Failed to get default GTK screen");
    StyleContext::add_provider_for_screen(&screen, &provider, 800);

    let (watcher_tx, watcher_rx) = mpsc::channel::<DebouncedEvent>();
    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    spawn(async move {
        match notify::watcher(watcher_tx, Duration::from_millis(500)) {
            Ok(mut watcher) => {
                watcher
                    .watch(&style_path, RecursiveMode::NonRecursive)
                    .expect("Unexpected error when attempting to watch CSS");

                loop {
                    if let Ok(DebouncedEvent::Write(path)) = watcher_rx.recv() {
                        tx.send(path).expect("Failed to send style changed message");
                    }
                }
            }
            Err(err) => error!(
                "{:?}",
                Report::new(err).wrap_err("Failed to start CSS watcher")
            ),
        }
    });

    {
        rx.attach(None, move |path| {
            info!("Reloading CSS");
            if let Err(err) = provider
                .load_from_file(&gio::File::for_path(path)) {
                error!("{:?}", Report::new(err)
                    .wrap_err("Failed to load CSS")
                    .suggestion("Check the CSS file for errors")
                    .suggestion("GTK CSS uses a subset of the full CSS spec and many properties are not available. Ensure you are not using any unsupported property.")
                );
            }

            Continue(true)
        });
    }
}
