mod bar;
mod bridge_channel;
mod clients;
mod config;
mod dynamic_string;
mod icon;
mod logging;
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
use std::future::Future;
use std::path::PathBuf;
use std::process::exit;
use std::{env, panic};
use tokio::runtime::Handle;
use tokio::task::block_in_place;

use crate::logging::install_tracing;
use clients::wayland::{self, WaylandClient};
use tracing::{debug, error, info};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[repr(i32)]
enum ErrorCode {
    GtkDisplay = 1,
    CreateBars = 2,
    Config = 3,
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

    let hook_builder = color_eyre::config::HookBuilder::default();
    let (panic_hook, eyre_hook) = hook_builder.into_hooks();

    eyre_hook.install()?;

    // custom hook allows tracing_appender to capture panics
    panic::set_hook(Box::new(move |panic_info| {
        error!("{}", panic_hook.panic_report(panic_info));
    }));

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
                exit(ErrorCode::GtkDisplay as i32)
            },
            |display| display,
        );

        let config = match Config::load() {
            Ok(config) => config,
            Err(err) => {
                error!("{:?}", err);
                exit(ErrorCode::Config as i32)
            }
        };
        debug!("Loaded config file");

        if let Err(err) = create_bars(app, &display, wayland_client, &config) {
            error!("{:?}", err);
            exit(ErrorCode::CreateBars as i32);
        }

        debug!("Created bars");

        let style_path = env::var("IRONBAR_CSS").ok().map_or_else(
            || {
                config_dir().map_or_else(
                    || {
                        let report = Report::msg("Failed to locate user config dir");
                        error!("{:?}", report);
                        exit(ErrorCode::CreateBars as i32);
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
///
/// TODO: remove all instances of this once async trait funcs are stable
pub fn await_sync<F: Future>(f: F) -> F::Output {
    block_in_place(|| Handle::current().block_on(f))
}
