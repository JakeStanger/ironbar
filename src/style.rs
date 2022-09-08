use color_eyre::{Help, Report};
use glib::Continue;
use gtk::prelude::CssProviderExt;
use gtk::{gdk, gio, CssProvider, StyleContext};
use notify::{Event, RecursiveMode, Result, Watcher};
use std::path::PathBuf;
use tokio::spawn;
use tracing::{error, info};

/// Attempts to load CSS file at the given path
/// and attach if to the current GTK application.
///
/// Installs a file watcher and reloads CSS when
/// write changes are detected on the file.
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

    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    spawn(async move {
        match notify::recommended_watcher(move |res: Result<Event>| match res {
            Ok(event) => {
                if let Some(path) = event.paths.first() {
                    tx.send(path.clone())
                        .expect("Failed to send style changed message");
                }
            }
            Err(e) => error!("Error occurred when watching stylesheet: {:?}", e),
        }) {
            Ok(mut watcher) => {
                watcher
                    .watch(&style_path, RecursiveMode::NonRecursive)
                    .expect("Unexpected error when attempting to watch CSS");
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
