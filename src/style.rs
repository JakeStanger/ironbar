use crate::{glib_recv_mpsc, spawn, try_send};
use color_eyre::{Help, Report};
use gtk::ffi::GTK_STYLE_PROVIDER_PRIORITY_USER;
use gtk::prelude::*;
use gtk::{Application, CssProvider, StyleContext, gdk, gio};
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecursiveMode, Result, Watcher, recommended_watcher};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info};

/// Attempts to load CSS file at the given path
/// and attach if to the current GTK application.
///
/// Installs a file watcher and reloads CSS when
/// write changes are detected on the file.
pub fn load_css(style_path: PathBuf, application: Application) {
    // file watcher requires absolute path
    let style_path = if style_path.is_absolute() {
        style_path
    } else {
        env::current_dir().expect("to exist").join(style_path)
    };

    let provider = CssProvider::new();
    provider.load_from_file(&gio::File::for_path(&style_path));
    debug!("Loaded css from '{}'", style_path.display());

    // GTK4 deprecates style contexts and loading custom styles
    // When GTK5 comes around, this will be gone for good.
    // For now though, our only option is to use this deprecated method.
    #[allow(deprecated)]
    StyleContext::add_provider_for_display(
        &crate::get_display(),
        &provider,
        GTK_STYLE_PROVIDER_PRIORITY_USER as u32,
    );

    let (tx, rx) = mpsc::channel(8);

    spawn(async move {
        let style_path2 = style_path.clone();
        let mut watcher = recommended_watcher(move |res: Result<Event>| match res {
            Ok(event) if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) => {
                debug!("{event:?}");
                if event.paths.first().is_some_and(|p| p == &style_path2) {
                    try_send!(tx, style_path2.clone());
                }
            }
            Err(e) => error!("Error occurred when watching stylesheet: {:?}", e),
            _ => {}
        })
        .expect("Failed to create CSS file watcher");

        let dir_path = style_path.parent().expect("to exist");

        watcher
            .watch(dir_path, RecursiveMode::NonRecursive)
            .expect("Failed to start CSS file watcher");
        debug!("Installed CSS file watcher on '{}'", style_path.display());

        // avoid watcher from dropping
        loop {
            sleep(Duration::from_secs(1)).await;
        }
    });

    glib_recv_mpsc!(rx, path => {
        info!("Reloading CSS");
        provider.load_from_file(&gio::File::for_path(path));

        // TODO: Check if still needed
        for win in application.windows() {
            win.queue_draw();
        }
    });
}
