#![doc = include_str!("../README.md")]

use std::cell::RefCell;
use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
#[cfg(feature = "ipc")]
use std::sync::{Arc, RwLock};

use cfg_if::cfg_if;
#[cfg(feature = "cli")]
use clap::Parser;
use color_eyre::eyre::Result;
use color_eyre::Report;
use dirs::config_dir;
use glib::PropertySet;
use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::Application;
use lazy_static::lazy_static;
use tokio::runtime::Handle;
use tokio::task::{block_in_place, spawn_blocking};
use tracing::{debug, error, info, warn};
use universal_config::ConfigLoader;

use clients::wayland;

use crate::bar::{create_bar, Bar};
use crate::config::{Config, MonitorConfig};
use crate::error::ExitCode;
#[cfg(feature = "ipc")]
use crate::ironvar::VariableManager;
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

const GTK_APP_ID: &str = "dev.jstanger.ironbar";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let _guard = logging::install_logging();

    cfg_if! {
        if #[cfg(feature = "cli")] {
            run_with_args().await;
        } else {
            start_ironbar();
        }
    }
}

#[cfg(feature = "cli")]
async fn run_with_args() {
    let args = cli::Args::parse();

    match args.command {
        Some(command) => {
            let ipc = ipc::Ipc::new();
            match ipc.send(command).await {
                Ok(res) => cli::handle_response(res),
                Err(err) => error!("{err:?}"),
            };
        }
        None => start_ironbar(),
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(1);

#[cfg(feature = "ipc")]
lazy_static! {
    static ref VARIABLE_MANAGER: Arc<RwLock<VariableManager>> = arc_rw!(VariableManager::new());
}

#[derive(Debug)]
pub struct Ironbar {
    bars: Rc<RefCell<Vec<Bar>>>,
}

impl Ironbar {
    fn new() -> Self {
        Self {
            bars: Rc::new(RefCell::new(vec![])),
        }
    }

    fn start(self) {
        info!("Ironbar version {}", VERSION);
        info!("Starting application");

        let app = Application::builder().application_id(GTK_APP_ID).build();

        let running = AtomicBool::new(false);

        let instance = Rc::new(self);

        // force start wayland client ahead of ui
        let wl = wayland::get_client();
        lock!(wl).roundtrip();

        app.connect_activate(move |app| {
            if running.load(Ordering::Relaxed) {
                info!("Ironbar already running, returning");
                return;
            }

            running.set(true);

            cfg_if! {
                if #[cfg(feature = "ipc")] {
                    let ipc = ipc::Ipc::new();
                    ipc.start(app, instance.clone());
                }
            }

            *instance.bars.borrow_mut() = load_interface(app);

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

            // TODO: Start wayland client - listen for outputs
            //  All bar loading should happen as an event response to this
        });

        // Ignore CLI args
        // Some are provided by swaybar_config but not currently supported
        app.run_with_args(&Vec::<&str>::new());
    }

    /// Gets a `usize` ID value that is unique to the entire Ironbar instance.
    /// This is just a static `AtomicUsize` that increments every time this function is called.
    pub fn unique_id() -> usize {
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Gets the `Ironvar` manager singleton.
    #[cfg(feature = "ipc")]
    #[must_use]
    pub fn variable_manager() -> Arc<RwLock<VariableManager>> {
        VARIABLE_MANAGER.clone()
    }

    /// Gets a clone of a bar by its unique name.
    ///
    /// Since the bar contains mostly GTK objects,
    /// the clone is cheap enough to not worry about.
    #[must_use]
    pub fn bar_by_name(&self, name: &str) -> Option<Bar> {
        self.bars
            .borrow()
            .iter()
            .find(|&bar| bar.name() == name)
            .cloned()
    }
}

fn start_ironbar() {
    let ironbar = Ironbar::new();
    ironbar.start();
}

/// Loads the Ironbar config and interface.
pub fn load_interface(app: &Application) -> Vec<Bar> {
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
        let variable_manager = Ironbar::variable_manager();
        for (k, v) in ironvars {
            if write_lock!(variable_manager).set(k.clone(), v).is_err() {
                warn!("Ignoring invalid ironvar: '{k}'");
            }
        }
    }

    match create_bars(app, &display, &config) {
        Ok(bars) => {
            debug!("Created {} bars", bars.len());
            bars
        }
        Err(err) => {
            error!("{:?}", err);
            exit(ExitCode::CreateBars as i32);
        }
    }
}

/// Creates each of the bars across each of the (configured) outputs.
fn create_bars(app: &Application, display: &Display, config: &Config) -> Result<Vec<Bar>> {
    let wl = wayland::get_client();
    let outputs = lock!(wl).get_outputs();

    debug!("Received {} outputs from Wayland", outputs.len());
    debug!("Outputs: {:?}", outputs);

    let num_monitors = display.n_monitors();

    let mut all_bars = vec![];
    for i in 0..num_monitors {
        let monitor = display
            .monitor(i)
            .ok_or_else(|| Report::msg(error::ERR_OUTPUTS))?;
        let output = outputs
            .get(i as usize)
            .ok_or_else(|| Report::msg(error::ERR_OUTPUTS))?;

        let Some(monitor_name) = &output.name else {
            continue;
        };

        let mut bars = match config
            .monitors
            .as_ref()
            .and_then(|config| config.get(monitor_name))
        {
            Some(MonitorConfig::Single(config)) => {
                vec![create_bar(
                    app,
                    &monitor,
                    monitor_name.to_string(),
                    config.clone(),
                )?]
            }
            Some(MonitorConfig::Multiple(configs)) => configs
                .iter()
                .map(|config| create_bar(app, &monitor, monitor_name.to_string(), config.clone()))
                .collect::<Result<_>>()?,
            None => vec![create_bar(
                app,
                &monitor,
                monitor_name.to_string(),
                config.clone(),
            )?],
        };

        all_bars.append(&mut bars);
    }

    Ok(all_bars)
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
