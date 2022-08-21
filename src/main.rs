mod bar;
mod collection;
mod config;
mod icon;
mod logging;
mod modules;
mod popup;
mod style;
mod sway;

use crate::bar::create_bar;
use crate::config::{Config, MonitorConfig};
use crate::style::load_css;
use crate::sway::{get_client_error, SwayOutput};
use color_eyre::eyre::Result;
use color_eyre::Report;
use dirs::config_dir;
use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::Application;
use ksway::client::Client;
use ksway::IpcCommand;
use std::env;
use std::process::exit;

use crate::logging::install_tracing;
use tracing::{debug, error, info};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // Disable backtraces by default
    if env::var("RUST_LIB_BACKTRACE").is_err() {
        env::set_var("RUST_LIB_BACKTRACE", "0");
    }

    // keep guard in scope
    // otherwise file logging drops
    let _guard = install_tracing()?;

    color_eyre::install()?;

    info!("Ironbar version {}", VERSION);
    info!("Starting application");

    let app = Application::builder()
        .application_id("dev.jstanger.ironbar")
        .build();

    app.connect_activate(move |app| {
        let display = match Display::default() {
            Some(display) => display,
            None => {
                let report = Report::msg("Failed to get default GTK display");
                error!("{:?}", report);
                exit(1)
            }
        };

        let config = match Config::load() {
            Ok(config) => config,
            Err(err) => {
                error!("{:?}", err);
                Config::default()
            }
        };
        debug!("Loaded config file");

        if let Err(err) = create_bars(app, &display, &config) {
            error!("{:?}", err);
            exit(2);
        }

        debug!("Created bars");

        let style_path = match config_dir() {
            Some(dir) => dir.join("ironbar").join("style.css"),
            None => {
                let report = Report::msg("Failed to locate user config dir");
                error!("{:?}", report);
                exit(3);
            }
        };

        if style_path.exists() {
            load_css(style_path);
            debug!("Loaded CSS watcher file");
        }
    });

    app.run();

    Ok(())
}

fn create_bars(app: &Application, display: &Display, config: &Config) -> Result<()> {
    let mut sway_client = match Client::connect() {
        Ok(client) => Ok(client),
        Err(err) => Err(get_client_error(err)),
    }?;

    let outputs = match sway_client.ipc(IpcCommand::GetOutputs) {
        Ok(outputs) => Ok(outputs),
        Err(err) => Err(get_client_error(err)),
    }?;

    let outputs = serde_json::from_slice::<Vec<SwayOutput>>(&outputs)?;

    let num_monitors = display.n_monitors();

    for i in 0..num_monitors {
        let monitor = display.monitor(i).ok_or_else(|| Report::msg("GTK and Sway are reporting a different number of outputs - this is a severe bug and should never happen"))?;
        let monitor_name = &outputs.get(i as usize).ok_or_else(|| Report::msg("GTK and Sway are reporting a different set of outputs - this is a severe bug and should never happen"))?.name;

        config.monitors.as_ref().map_or_else(
            || create_bar(app, &monitor, monitor_name, config.clone()),
            |config| {
                let config = config.get(monitor_name);
                match &config {
                    Some(MonitorConfig::Single(config)) => {
                        create_bar(app, &monitor, monitor_name, config.clone())
                    }
                    Some(MonitorConfig::Multiple(configs)) => {
                        for config in configs {
                            create_bar(app, &monitor, monitor_name, config.clone())?;
                        }

                        Ok(())
                    }
                    _ => Ok(()),
                }
            },
        )?;
    }

    Ok(())
}
