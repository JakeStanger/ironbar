use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::spawn;
use gtk::ffi::GTK_STYLE_PROVIDER_PRIORITY_USER;
use gtk::{CssProvider, gio};
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecursiveMode, Result, Watcher, recommended_watcher};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info};

#[derive(Debug)]
pub enum CssSource {
    String(&'static str),
    File(PathBuf),
}

/// Attempts to load CSS file at the given path
/// and attach if to the current GTK application.
///
/// Installs a file watcher and reloads CSS when
/// write changes are detected on the file.
pub fn load_css(source: &CssSource, hot_reload: bool) {
    let provider = CssProvider::new();

    let path = match source {
        CssSource::String(str) => {
            provider.load_from_string(str);
            debug!("loaded built-in css");
            None
        }
        CssSource::File(style_path) => {
            // file watcher requires absolute path
            let style_path = if style_path.is_absolute() {
                style_path.to_path_buf()
            } else {
                env::current_dir().expect("to exist").join(style_path)
            };

            provider.load_from_path(&style_path);
            debug!("loaded css from '{}'", style_path.display());
            Some(style_path)
        }
    };

    // Deprecation warning is an error in gtk-rs bindings
    // <https://github.com/gtk-rs/gtk4-rs/pull/2161>
    #[allow(deprecated)]
    gtk::StyleContext::add_provider_for_display(
        &crate::get_display(),
        &provider,
        GTK_STYLE_PROVIDER_PRIORITY_USER as u32,
    );

    // install file watcher
    if hot_reload && let Some(style_path) = path {
        let (tx, rx) = mpsc::channel(8);

        spawn(async move {
            let style_path2 = style_path.clone();
            let mut watcher = recommended_watcher(move |res: Result<Event>| match res {
                Ok(event) if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) => {
                    debug!("{event:?}");
                    if event.paths.first().is_some_and(|p| p == &style_path2) {
                        tx.send_spawn(style_path2.clone());
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

        rx.recv_glib((), move |(), path| {
            info!("Reloading CSS");
            provider.load_from_file(&gio::File::for_path(path));
        });
    }
}
