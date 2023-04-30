#![doc = include_str!("../README.md")]

mod bar;
mod bridge_channel;
mod clients;
mod config;
mod desktop_file;
mod dynamic_string;
mod error;
mod image;
mod logging;
mod macros;
mod modules;
mod popup;
mod script;
mod style;

use crate::bar::create_bar;
use crate::config::{Config, MonitorConfig};
use crate::style::load_css;
use color_eyre::eyre::Result;
use color_eyre::Report;
use dirs::config_dir;
use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::Application;
use std::cell::Cell;
use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use tokio::runtime::Handle;
use tokio::task::block_in_place;

use crate::error::ExitCode;
use clients::wayland::{self, WaylandClient};
use tracing::{debug, error, info};
use universal_config::ConfigLoader;

const GTK_APP_ID: &str = "dev.jstanger.ironbar";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let _guard = logging::install_logging();

    info!("Ironbar version {}", VERSION);
    info!("Starting application");

    let wayland_client = wayland::get_client().await;

    let app = Application::builder().application_id(GTK_APP_ID).build();

    let running = Rc::new(Cell::new(false));

    app.connect_activate(move |app| {
        if running.get() {
            info!("Ironbar already running, returning");
            return;
        }

        running.set(true);

        let display = Display::default().map_or_else(
            || {
                let report = Report::msg("Failed to get default GTK display");
                error!("{:?}", report);
                exit(ExitCode::GtkDisplay as i32)
            },
            |display| display,
        );

        let config_res = env::var("IRONBAR_CONFIG").map_or_else(
            |_| ConfigLoader::new("ironbar").find_and_load(),
            ConfigLoader::load,
        );

        let config = match config_res {
            Ok(config) => config,
            Err(err) => {
                error!("{:?}", err);
                exit(ExitCode::Config as i32)
            }
        };

        debug!("Loaded config file");

        if let Err(err) = create_bars(app, &display, wayland_client, &config) {
            error!("{:?}", err);
            exit(ExitCode::CreateBars as i32);
        }

        debug!("Created bars");

        let style_path = env::var("IRONBAR_CSS").ok().map_or_else(
            || {
                config_dir().map_or_else(
                    || {
                        let report = Report::msg("Failed to locate user config dir");
                        error!("{:?}", report);
                        exit(ExitCode::CreateBars as i32);
                    },
                    |dir| dir.join("ironbar").join("style.css"),
                )
            },
            PathBuf::from,
        );

        if style_path.exists() {
            load_css(style_path);
        }
    });

    // Ignore CLI args
    // Some are provided by swaybar_config but not currently supported
    app.run_with_args(&Vec::<&str>::new());

    info!("Shutting down");
    exit(0);
}

/// Creates each of the bars across each of the (configured) outputs.
fn create_bars(
    app: &Application,
    display: &Display,
    wl: &WaylandClient,
    config: &Config,
) -> Result<()> {
    let outputs = wl.outputs.as_slice();

    debug!("Received {} outputs from Wayland", outputs.len());
    debug!("Outputs: {:?}", outputs);

    let num_monitors = display.n_monitors();

    for i in 0..num_monitors {
        let monitor = display
            .monitor(i)
            .ok_or_else(|| Report::msg(error::ERR_OUTPUTS))?;
        let output = outputs
            .get(i as usize)
            .ok_or_else(|| Report::msg(error::ERR_OUTPUTS))?;
        let monitor_name = &output.name;

        config.monitors.as_ref().map_or_else(
            || {
                info!("Creating bar on '{}'", monitor_name);
                create_bar(app, &monitor, monitor_name, config.clone())
            },
            |config| {
                let config = config.get(monitor_name);
                match &config {
                    Some(MonitorConfig::Single(config)) => {
                        info!("Creating bar on '{}'", monitor_name);
                        create_bar(app, &monitor, monitor_name, config.clone())
                    }
                    Some(MonitorConfig::Multiple(configs)) => {
                        for config in configs {
                            info!("Creating bar on '{}'", monitor_name);
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

/// Blocks on a `Future` until it resolves.
///
/// This is not an `async` operation
/// so can be used outside of an async function.
///
/// Do note it must be called from within a Tokio runtime still.
///
/// Use sparingly! Prefer async functions wherever possible.
///
/// TODO: remove all instances of this once async trait funcs are stable
pub fn await_sync<F: Future>(f: F) -> F::Output {
    block_in_place(|| Handle::current().block_on(f))
}
