mod bar;
mod collection;
mod config;
mod icon;
mod modules;
mod popup;
mod style;
mod sway;

use crate::bar::create_bar;
use crate::config::{Config, MonitorConfig};
use crate::style::load_css;
use crate::sway::SwayOutput;
use dirs::config_dir;
use gtk::prelude::*;
use gtk::{gdk, Application};
use ksway::client::Client;
use ksway::IpcCommand;

#[tokio::main]
async fn main() {
    let app = Application::builder()
        .application_id("dev.jstanger.waylandbar")
        .build();

    let mut sway_client = Client::connect().expect("Failed to connect to Sway IPC");
    let outputs = sway_client
        .ipc(IpcCommand::GetOutputs)
        .expect("Failed to get Sway outputs");
    let outputs = serde_json::from_slice::<Vec<SwayOutput>>(&outputs)
        .expect("Failed to deserialize outputs message from Sway IPC");

    app.connect_activate(move |app| {
        let config = Config::load().unwrap_or_default();

        // TODO: Better logging (https://crates.io/crates/tracing)
        // TODO: error handling (https://crates.io/crates/color-eyre)

        // TODO: Embedded Deno/lua - build custom modules via script???

        let display = gdk::Display::default().expect("Failed to get default GDK display");
        let num_monitors = display.n_monitors();

        for i in 0..num_monitors {
            let monitor = display.monitor(i).unwrap();
            let monitor_name = &outputs
                .get(i as usize)
                .expect("GTK monitor output differs from Sway's")
                .name;

            config.monitors.as_ref().map_or_else(
                || {
                    create_bar(app, &monitor, monitor_name, config.clone());
                },
                |config| {
                    let config = config.get(monitor_name);
                    match &config {
                        Some(MonitorConfig::Single(config)) => {
                            create_bar(app, &monitor, monitor_name, config.clone());
                        }
                        Some(MonitorConfig::Multiple(configs)) => {
                            for config in configs {
                                create_bar(app, &monitor, monitor_name, config.clone());
                            }
                        }
                        _ => {}
                    }
                },
            )
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
