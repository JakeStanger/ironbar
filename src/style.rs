use crate::{get_display, glib_recv_mpsc, spawn, try_send};
use gtk::ffi::GTK_STYLE_PROVIDER_PRIORITY_USER;
use gtk::{gio, CssProvider};
use notify::event::ModifyKind;
use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Result, Watcher};
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
pub fn load_css(style_path: PathBuf) {
    // file watcher requires absolute path
    let style_path = if style_path.is_absolute() {
        style_path
    } else {
        env::current_dir().expect("to exist").join(style_path)
    };

    let provider = CssProvider::new();
    provider.load_from_file(&gio::File::for_path(&style_path));

    gtk::style_context_add_provider_for_display(
        &get_display(),
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
    });
}
