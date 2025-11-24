#![doc = include_str!("../README.md")]

use std::cell::RefCell;
use std::env;
use std::future::Future;
use std::process::exit;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, mpsc};

use cfg_if::cfg_if;
#[cfg(feature = "cli")]
use clap::Parser;
use color_eyre::{Report, Result};
use gtk::Application;
use gtk::gdk::{Display, Monitor};
use gtk::prelude::*;
use smithay_client_toolkit::output::OutputInfo;
use tokio::runtime::Runtime;
use tokio::task::{JoinHandle, block_in_place};
use tracing::{debug, error, info};

use crate::bar::{Bar, create_bar};
use crate::channels::SyncSenderExt;
use crate::clients::Clients;
use crate::clients::outputs::MonitorState;
use crate::config::{Config, ConfigLocation, MonitorConfig};
use crate::desktop_file::DesktopFiles;
use crate::error::ExitCode;
#[cfg(feature = "ipc")]
use crate::ironvar::VariableManager;
use crate::style::{CssSource, load_css};

mod bar;
mod channels;
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

pub const APP_ID: &str = "dev.jstanger.ironbar";
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    cfg_if! {
        if #[cfg(feature = "cli")] {
            run_with_args();
        } else {
            let config_location = ConfigLocation::from_env("IRONBAR_CONFIG").unwrap_or_default();
            start_ironbar(false, config_location, ConfigLocation::from_env("IRONBAR_CSS"));
        }
    }
}

#[cfg(feature = "cli")]
fn run_with_args() {
    let args = cli::Args::parse();

    #[cfg(feature = "extras")]
    if args.print_schema {
        let schema = schemars::schema_for!(Config);
        println!(
            "{}",
            serde_json::to_string_pretty(&schema).expect("to be serializable")
        );
        return;
    }

    #[cfg(feature = "extras")]
    if let Some(shell) = args.print_completions {
        use clap::CommandFactory;

        let mut cmd = cli::Args::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(
            clap_complete::Shell::from(shell),
            &mut cmd,
            name,
            &mut std::io::stdout(),
        );
        return;
    }

    match args.command {
        Some(command) => {
            if args.debug {
                eprintln!("REQUEST: {command:?}");
            }

            let rt = create_runtime();
            rt.block_on(async move {
                let ipc = ipc::Ipc::new();
                match ipc.send(command, args.debug).await {
                    Ok(res) => {
                        if args.debug {
                            eprintln!("RESPONSE: {res:?}");
                        }

                        cli::handle_response(res, args.format.unwrap_or_default());
                    }
                    Err(err) => {
                        error!("{err:#}");
                        exit(ExitCode::IpcResponseError as i32)
                    }
                }
            });
        }
        None => start_ironbar(args.debug, args.config.unwrap_or_default(), args.theme),
    }
}

#[derive(Debug)]
pub struct Ironbar {
    bars: Rc<RefCell<Vec<Bar>>>,
    clients: Rc<RefCell<Clients>>,
    config: Rc<RefCell<Config>>,
    css_source: Rc<CssSource>,
    config_location: ConfigLocation,
    css_location: Option<ConfigLocation>,

    desktop_files: DesktopFiles,
    image_provider: image::Provider,
}

impl Ironbar {
    fn new(config_location: ConfigLocation, css_location: Option<ConfigLocation>) -> Self {
        let (mut config, css_source) = Config::load(config_location.clone(), css_location.clone());

        let desktop_files = DesktopFiles::new();
        let image_provider =
            image::Provider::new(desktop_files.clone(), &mut config.icon_overrides);

        Self {
            bars: rc_mut!(vec![]),
            clients: rc_mut!(Clients::new()),
            config: rc_mut!(config),
            css_source: Rc::new(css_source),
            config_location,
            css_location,
            desktop_files,
            image_provider,
        }
    }

    fn start(self) {
        info!("Ironbar version {}", VERSION);
        info!("Starting application");

        let app = Application::builder().application_id(APP_ID).build();

        let running = AtomicBool::new(false);

        // cannot use `oneshot` as `connect_activate` is not `FnOnce`.
        let (activate_tx, activate_rx) = mpsc::channel();

        let css_source = self.css_source.clone();

        let instance = Rc::new(self);
        let instance2 = instance.clone();

        // force start wayland client ahead of ui
        let wl = instance.clients.borrow_mut().wayland();
        wl.roundtrip();

        app.connect_activate(move |app| {
            if running.load(Ordering::Relaxed) {
                info!("Ironbar already running, returning");
                return;
            }

            running.store(true, Ordering::Relaxed);

            cfg_if! {
                if #[cfg(feature = "ipc")] {
                    let ipc = ipc::Ipc::new();
                    ipc.start(app, instance.clone());
                }
            }

            load_css(&css_source);

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

            let hold = app.hold();
            activate_tx.send_expect(hold);
        });

        {
            let instance = instance2.clone();
            let app = app.clone();

            glib::spawn_future_local(async move {
                let _hold = activate_rx.recv().expect("to receive activation signal");
                debug!("Received activation signal, initialising bars");

                instance
                    .image_provider
                    .set_icon_theme(instance.config.borrow().icon_theme.as_deref());

                // Load initial bars
                match load_output_bars(&instance.clone(), &app) {
                    Ok(_) => {}
                    Err(report) => {
                        error!("{:?}", report);
                        exit(ExitCode::CreateBars as i32);
                    }
                };

                let outputs = instance.clients.borrow_mut().outputs();
                let mut rx_outputs = outputs.subscribe();

                outputs.start(&instance.clone());

                // Listen for monitor events
                while let Ok(event) = rx_outputs.recv().await {
                    match event.state {
                        MonitorState::Disconnected => {
                            instance
                                .bars
                                .borrow_mut()
                                .extract_if(.., |bar| bar.monitor_name() == event.connector)
                                .for_each(Bar::close);
                        }
                        MonitorState::Connected(wl_output, gdk_output) => {
                            if let Some(gdk_output) = gdk_output.upgrade() {
                                match load_output_bars_for(&instance, &app, &wl_output, &gdk_output)
                                {
                                    Ok(mut new_bars) => {
                                        instance.bars.borrow_mut().append(&mut new_bars);
                                    }
                                    Err(err) => error!("{err:?}"),
                                }
                            }
                        }
                    }
                }
            });
        }

        // Ignore CLI args
        // Some are provided by swaybar_config but not currently supported
        app.run_with_args(&Vec::<&str>::new());
    }

    /// Gets the current Tokio runtime.
    #[must_use]
    pub fn runtime() -> Arc<Runtime> {
        static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
        RUNTIME.get_or_init(|| Arc::new(create_runtime())).clone()
    }

    /// Gets a `usize` ID value that is unique to the entire Ironbar instance.
    /// This is just a static `AtomicUsize` that increments every time this function is called.
    #[must_use]
    pub fn unique_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Gets the `Ironvar` manager singleton.
    #[cfg(feature = "ipc")]
    #[must_use]
    pub fn variable_manager() -> Arc<VariableManager> {
        static VARIABLE_MANAGER: OnceLock<Arc<VariableManager>> = OnceLock::new();
        VARIABLE_MANAGER
            .get_or_init(|| Arc::new(VariableManager::new()))
            .clone()
    }

    #[must_use]
    pub fn desktop_files(&self) -> DesktopFiles {
        self.desktop_files.clone()
    }

    #[must_use]
    pub fn image_provider(&self) -> image::Provider {
        self.image_provider.clone()
    }

    /// Gets clones of bars by their name.
    ///
    /// Since the bars contain mostly GTK objects,
    /// the clone is cheap enough to not worry about.
    #[must_use]
    pub fn bars_by_name(&self, name: &str) -> Vec<Bar> {
        self.bars
            .borrow()
            .iter()
            .filter(|&bar| bar.name() == name)
            .cloned()
            .collect()
    }

    /// Re-reads the config file from disk and replaces the active config.
    /// Note this does *not* reload bars, which must be performed separately.
    #[cfg(feature = "ipc")]
    fn reload_config(&self) {
        self.config
            .replace(Config::load(self.config_location.clone(), self.css_location.clone()).0);
    }
}

fn start_ironbar(
    debug: bool,
    config_location: ConfigLocation,
    css_location: Option<ConfigLocation>,
) {
    let _guard = logging::install_logging(debug);

    let ironbar = Ironbar::new(config_location, css_location);
    ironbar.start();
}

/// Gets the GDK `Display` instance.
#[must_use]
pub fn get_display() -> Display {
    Display::default().map_or_else(
        || {
            let report = Report::msg("Failed to get default GTK display");
            error!("{:?}", report);
            exit(ExitCode::GtkDisplay as i32)
        },
        |display| display,
    )
}

/// Loads all the bars associated with an output.
fn load_output_bars_for(
    ironbar: &Rc<Ironbar>,
    app: &Application,
    output: &OutputInfo,
    monitor: &Monitor,
) -> Result<Vec<Bar>> {
    let Some(monitor_name) = &output.name else {
        return Err(Report::msg("Output missing monitor name"));
    };

    let monitor_desc = &output.description.clone().unwrap_or_default();

    let config = ironbar.config.borrow();

    let show_default_bar =
        config.bar.start.is_some() || config.bar.center.is_some() || config.bar.end.is_some();

    let bars = match config.monitors.as_ref().and_then(|config| {
        config.get(monitor_name).or_else(|| {
            config
                .keys()
                .find(|&k| monitor_desc.to_lowercase().starts_with(&k.to_lowercase()))
                .and_then(|key| config.get(key))
        })
    }) {
        Some(MonitorConfig::Single(config)) => {
            vec![create_bar(
                app,
                monitor,
                monitor_name.to_string(),
                config.clone(),
                ironbar.clone(),
            )]
        }
        Some(MonitorConfig::Multiple(configs)) => configs
            .iter()
            .map(|config| {
                create_bar(
                    app,
                    monitor,
                    monitor_name.to_string(),
                    config.clone(),
                    ironbar.clone(),
                )
            })
            .collect(),
        None if show_default_bar => vec![create_bar(
            app,
            monitor,
            monitor_name.to_string(),
            config.bar.clone(),
            ironbar.clone(),
        )],
        None => vec![],
    };

    Ok(bars)
}

pub fn load_output_bars(ironbar: &Rc<Ironbar>, app: &Application) -> Result<()> {
    let wl = ironbar.clients.borrow_mut().wayland();
    let outputs = wl.output_info_all();

    let display = get_display();
    let monitors = display.monitors();

    for output in outputs {
        let Some(monitor_name) = &output.name else {
            return Err(Report::msg("Output missing monitor name"));
        };
        let monitor_desc = &output.description.clone().unwrap_or_default();
        let find_monitor = || {
            for i in 0..monitors.n_items() {
                let Some(monitor) = monitors.item(i).and_downcast::<Monitor>() else {
                    continue;
                };

                if monitor.description().unwrap_or_default().as_str() == monitor_desc
                    || monitor.connector().unwrap_or_default().as_str() == monitor_name
                {
                    return Some(monitor);
                }
            }

            None
        };

        let Some(monitor) = find_monitor() else {
            error!("failed to find matching monitor for {}", monitor_name);
            continue;
        };

        match crate::load_output_bars_for(ironbar, app, &output, &monitor) {
            Ok(mut bars) => ironbar.bars.borrow_mut().append(&mut bars),
            Err(err) => error!("{err:?}"),
        }
    }

    Ok(())
}

fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio to create a valid runtime")
}

/// Calls `spawn` on the Tokio runtime.
pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    Ironbar::runtime().spawn(f)
}

/// Calls `spawn_blocking` on the Tokio runtime.
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    Ironbar::runtime().spawn_blocking(f)
}

/// Blocks on a `Future` until it resolves.
///
/// This is not an `async` operation
/// so can be used outside an async function.
///
/// Use sparingly, as this risks blocking the UI thread!
/// Prefer async functions wherever possible.
pub fn await_sync<F: Future>(f: F) -> F::Output {
    block_in_place(|| Ironbar::runtime().block_on(f))
}
