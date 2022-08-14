use glib::Continue;
use gtk::prelude::CssProviderExt;
use gtk::{gdk, gio, CssProvider, StyleContext};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tokio::spawn;

pub fn load_css(style_path: PathBuf) {
    let provider = CssProvider::new();
    provider
        .load_from_file(&gio::File::for_path(&style_path))
        .expect("Couldn't load custom style");
    StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Couldn't get default GDK screen"),
        &provider,
        800,
    );

    let (watcher_tx, watcher_rx) = mpsc::channel::<DebouncedEvent>();
    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    spawn(async move {
        let mut watcher = notify::watcher(watcher_tx, Duration::from_millis(500)).unwrap();
        watcher
            .watch(&style_path, RecursiveMode::NonRecursive)
            .unwrap();

        loop {
            if let Ok(DebouncedEvent::Write(path)) = watcher_rx.recv() {
                tx.send(path).unwrap();
            }
        }
    });

    {
        rx.attach(None, move |path| {
            println!("Reloading CSS");
            provider
                .load_from_file(&gio::File::for_path(path))
                .expect("Couldn't load custom style");

            Continue(true)
        });
    }
}
