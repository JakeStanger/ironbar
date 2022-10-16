mod bar;
mod bridge_channel;
mod collection;
mod config;
mod icon;
mod logging;
mod modules;
mod popup;
mod style;
mod sway;
mod wayland;

use crate::bar::create_bar;
use crate::config::{Config, MonitorConfig};
use crate::style::load_css;
use color_eyre::eyre::Result;
use color_eyre::Report;
use dirs::config_dir;
use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::Application;
use std::env;
use std::future::Future;
use std::process::exit;
use tokio::runtime::Handle;
use tokio::task::block_in_place;

use crate::logging::install_tracing;
use tracing::{debug, error, info};
use wayland::WaylandClient;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[repr(i32)]
enum ExitCode {
    ErrGtkDisplay = 1,
    ErrCreateBars = 2,
    ErrConfig = 3
}

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

    let wayland_client = wayland::get_client().await;

    let app = Application::builder()
        .application_id("dev.jstanger.ironbar")
        .build();

    app.connect_activate(move |app| {
        let display = Display::default().map_or_else(
            || {
                let report = Report::msg("Failed to get default GTK display");
                error!("{:?}", report);
                exit(ExitCode::ErrGtkDisplay as i32)
            },
            |display| display,
        );

        let config = match Config::load() {
            Ok(config) => config,
            Err(err) => {
                error!("{:?}", err);
                exit(ExitCode::ErrConfig as i32)
            }
        };
        debug!("Loaded config file");

        if let Err(err) = create_bars(app, &display, wayland_client, &config) {
            error!("{:?}", err);
            exit(ExitCode::ErrCreateBars as i32);
        }

        debug!("Created bars");

        let style_path = config_dir().map_or_else(
            || {
                let report = Report::msg("Failed to locate user config dir");
                error!("{:?}", report);
                exit(ExitCode::ErrCreateBars as i32);
            },
            |dir| dir.join("ironbar").join("style.css"),
        );

        if style_path.exists() {
            load_css(style_path);
            debug!("Loaded CSS watcher file");
        }
    });

    // Ignore CLI args
    // Some are provided by swaybar_config but not currently supported
    app.run_with_args(&Vec::<&str>::new());

    Ok(())
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
        let monitor = display.monitor(i).ok_or_else(|| Report::msg("GTK and Sway are reporting a different set of outputs - this is a severe bug and should never happen"))?;
        let output = outputs.get(i as usize).ok_or_else(|| Report::msg("GTK and Sway are reporting a different set of outputs - this is a severe bug and should never happen"))?;
        let monitor_name = &output.name;

        // TODO: Could we use an Arc<Config> or `Cow<Config>` here to avoid cloning?
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
pub fn await_sync<F: Future>(f: F) -> F::Output {
    block_in_place(|| Handle::current().block_on(f))
}
