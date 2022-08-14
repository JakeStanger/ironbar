mod bar;
mod collection;
mod config;
mod modules;
mod popup;
mod style;

use crate::bar::create_bar;
use crate::config::Config;
use crate::style::load_css;
use dirs::config_dir;
use gtk::prelude::*;
use gtk::{gdk, Application};

#[tokio::main]
async fn main() {
    let app = Application::builder()
        .application_id("dev.jstanger.waylandbar")
        .build();

    app.connect_activate(|app| {
        let config = Config::load().unwrap_or_default();

        // TODO: Better logging (https://crates.io/crates/tracing)
        // TODO: error handling (https://crates.io/crates/color-eyre)

        // TODO: Embedded Deno/lua - build custom modules via script???

        let display = gdk::Display::default().expect("Failed to get default GDK display");
        let num_monitors = display.n_monitors();
        for i in 0..num_monitors {
            let monitor = display.monitor(i).unwrap();

            let config = config.monitors.as_ref().map_or(&config, |monitor_config| {
                monitor_config.get(i as usize).unwrap_or(&config)
            });

            create_bar(app, &monitor, config.clone());
        }

        let style_path = config_dir()
            .expect("Failed to locate user config dir")
            .join("ironbar")
            .join("style.css");

        if style_path.exists() {
            load_css(style_path);
        }
    });

    app.run();
}
