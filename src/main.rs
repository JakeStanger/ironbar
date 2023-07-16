#![doc = include_str!("../README.md")]

use std::cell::{Cell, RefCell};
use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::mpsc;

use cfg_if::cfg_if;
#[cfg(feature = "cli")]
use clap::Parser;
use color_eyre::eyre::Result;
use color_eyre::Report;
use dirs::config_dir;
use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::Application;
use tokio::runtime::Handle;
use tokio::task::{block_in_place, spawn_blocking};
use tracing::{debug, error, info, warn};
use universal_config::ConfigLoader;

use clients::wayland;

use crate::bar::create_bar;
use crate::config::{Config, MonitorConfig};
use crate::error::ExitCode;
use crate::global_state::GlobalState;
use crate::style::load_css;

mod bar;
mod bridge_channel;
#[cfg(feature = "cli")]
mod cli;
mod clients;
mod config;
mod desktop_file;
mod dynamic_value;
mod error;
mod global_state;
mod gtk_helpers;
mod image;
#[cfg(feature = "ipc")]
mod ipc;
#[cfg(feature = "ipc")]
mod ironvar;
mod logging;
mod macros;
mod modules;
mod popup;
mod script;
mod style;
mod unique_id;

const GTK_APP_ID: &str = "dev.jstanger.ironbar";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let _guard = logging::install_logging();

    let global_state = Rc::new(RefCell::new(GlobalState::new()));

    cfg_if! {
        if #[cfg(feature = "cli")] {
            run_with_args(global_state).await;
        } else {
            start_ironbar(global_state);
        }
    }
}

#[cfg(feature = "cli")]
async fn run_with_args(global_state: Rc<RefCell<GlobalState>>) {
    let args = cli::Args::parse();

    match args.command {
        Some(command) => {
            let ipc = ipc::Ipc::new(global_state);
            match ipc.send(command).await {
                Ok(res) => cli::handle_response(res),
                Err(err) => error!("{err:?}"),
            };
        }
        None => start_ironbar(global_state),
    }
}

fn start_ironbar(global_state: Rc<RefCell<GlobalState>>) {
    info!("Ironbar version {}", VERSION);
    info!("Starting application");

    let app = Application::builder().application_id(GTK_APP_ID).build();
    let _ = wayland::get_client(); // force-init

    let running = Rc::new(Cell::new(false));

    app.connect_activate(move |app| {
        if running.get() {
            info!("Ironbar already running, returning");
            return;
        }

        running.set(true);

        cfg_if! {
            if #[cfg(feature = "ipc")] {
                let ipc = ipc::Ipc::new(global_state.clone());
                ipc.start(app);
            }
        }

        load_interface(app, &global_state);

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

        let (tx, rx) = mpsc::channel();

        #[cfg(feature = "ipc")]
        let ipc_path = ipc.path().to_path_buf();
        spawn_blocking(move || {
            rx.recv().expect("to receive from channel");

            info!("Shutting down");

            #[cfg(feature = "ipc")]
            ipc::Ipc::shutdown(ipc_path);

            exit(0);
        });

        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
            .expect("Error setting Ctrl-C handler");
    });

    // Ignore CLI args
    // Some are provided by swaybar_config but not currently supported
    app.run_with_args(&Vec::<&str>::new());
}

/// Loads the Ironbar config and interface.
pub fn load_interface(app: &Application, global_state: &Rc<RefCell<GlobalState>>) {
    let display = Display::default().map_or_else(
        || {
            let report = Report::msg("Failed to get default GTK display");
            error!("{:?}", report);
            exit(ExitCode::GtkDisplay as i32)
        },
        |display| display,
    );

    let mut config = env::var("IRONBAR_CONFIG")
        .map_or_else(
            |_| ConfigLoader::new("ironbar").find_and_load(),
            ConfigLoader::load,
        )
        .unwrap_or_else(|err| {
            error!("Failed to load config: {}", err);
            warn!("Falling back to the default config");
            info!("If this is your first time using Ironbar, you should create a config in ~/.config/ironbar/");
            info!("More info here: https://github.com/JakeStanger/ironbar/wiki/configuration-guide");

            Config::default()
        });

    debug!("Loaded config file");

    #[cfg(feature = "ipc")]
    if let Some(ironvars) = config.ironvar_defaults.take() {
        let variable_manager = ironvar::get_variable_manager();
        for (k, v) in ironvars {
            if write_lock!(variable_manager).set(k.clone(), v).is_err() {
                warn!("Ignoring invalid ironvar: '{k}'");
            }
        }
    }

    if let Err(err) = create_bars(app, &display, &config, global_state) {
        error!("{:?}", err);
        exit(ExitCode::CreateBars as i32);
    }

    debug!("Created bars");
}

/// Creates each of the bars across each of the (configured) outputs.
fn create_bars(
    app: &Application,
    display: &Display,
    config: &Config,
    global_state: &Rc<RefCell<GlobalState>>,
) -> Result<()> {
    let wl = wayland::get_client();
    let outputs = lock!(wl).get_outputs();

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

        let Some(monitor_name) = &output.name else { continue };

        config.monitors.as_ref().map_or_else(
            || {
                info!("Creating bar on '{}'", monitor_name);
                create_bar(app, &monitor, monitor_name, config.clone(), global_state)
            },
            |config| {
                let config = config.get(monitor_name);
                match &config {
                    Some(MonitorConfig::Single(config)) => {
                        info!("Creating bar on '{}'", monitor_name);
                        create_bar(app, &monitor, monitor_name, config.clone(), global_state)
                    }
                    Some(MonitorConfig::Multiple(configs)) => {
                        for config in configs {
                            info!("Creating bar on '{}'", monitor_name);
                            create_bar(app, &monitor, monitor_name, config.clone(), global_state)?;
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
