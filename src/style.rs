use crate::error as err;
use color_eyre::{Help, Report};
use glib::Continue;
use gtk::prelude::CssProviderExt;
use gtk::{gdk, gio, CssProvider, StyleContext};
use notify::event::{DataChange, ModifyKind};
use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Result, Watcher};
use std::path::PathBuf;
use std::time::Duration;
use tokio::spawn;
use tokio::time::sleep;
use tracing::{debug, error, info};

/// Attempts to load CSS file at the given path
/// and attach if to the current GTK application.
///
/// Installs a file watcher and reloads CSS when
/// write changes are detected on the file.
pub fn load_css(style_path: PathBuf) {
    let provider = CssProvider::new();

    match provider.load_from_file(&gio::File::for_path(&style_path)) {
        Ok(()) => debug!("Loaded css from '{}'", style_path.display()),
        Err(err) => error!("{:?}", Report::new(err)
                    .wrap_err("Failed to load CSS")
                    .suggestion("Check the CSS file for errors")
                    .suggestion("GTK CSS uses a subset of the full CSS spec and many properties are not available. Ensure you are not using any unsupported property.")
                )
    };

    let screen = gdk::Screen::default().expect("Failed to get default GTK screen");
    StyleContext::add_provider_for_screen(&screen, &provider, 800);

    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    spawn(async move {
        let mut watcher = recommended_watcher(move |res: Result<Event>| match res {
            Ok(event) if event.kind == EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {
                debug!("{event:?}");
                if let Some(path) = event.paths.first() {
                    tx.send(path.clone()).expect(err::ERR_CHANNEL_SEND);
                }
            }
            Err(e) => error!("Error occurred when watching stylesheet: {:?}", e),
            _ => {}
        })
        .expect("Failed to create CSS file watcher");

        watcher
            .watch(&style_path, RecursiveMode::NonRecursive)
            .expect("Failed to start CSS file watcher");
        debug!("Installed CSS file watcher on '{}'", style_path.display());

        // avoid watcher from dropping
        loop {
            sleep(Duration::from_secs(1)).await;
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
