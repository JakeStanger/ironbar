use color_eyre::Result;
use dirs::data_dir;
use std::{env, panic};
use strip_ansi_escapes::Writer;
use tracing::error;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::Rotation;
use tracing_error::ErrorLayer;
use tracing_subscriber::fmt::{Layer, MakeWriter};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

struct MakeFileWriter {
    file_writer: NonBlocking,
}

impl MakeFileWriter {
    const fn new(file_writer: NonBlocking) -> Self {
        Self { file_writer }
    }
}

impl<'a> MakeWriter<'a> for MakeFileWriter {
    type Writer = Writer<NonBlocking>;

    fn make_writer(&'a self) -> Self::Writer {
        Writer::new(self.file_writer.clone())
    }
}

pub fn install_logging() -> Result<WorkerGuard> {
    // Disable backtraces by default
    if env::var("RUST_LIB_BACKTRACE").is_err() {
        env::set_var("RUST_LIB_BACKTRACE", "0");
    }

    // keep guard in scope
    // otherwise file logging drops
    let guard = install_tracing()?;

    let hook_builder = color_eyre::config::HookBuilder::default();
    let (panic_hook, eyre_hook) = hook_builder.into_hooks();

    eyre_hook.install()?;

    // custom hook allows tracing_appender to capture panics
    panic::set_hook(Box::new(move |panic_info| {
        error!("{}", panic_hook.panic_report(panic_info));
    }));

    Ok(guard)
}

/// Installs tracing into the current application.
///
/// The returned `WorkerGuard` must remain in scope
/// for the lifetime of the application for logging to file to work.
fn install_tracing() -> Result<WorkerGuard> {
    const DEFAULT_LOG: &str = "info";
    const DEFAULT_FILE_LOG: &str = "warn";

    let fmt_layer = fmt::layer().with_target(true).with_line_number(true);
    let filter_layer =
        EnvFilter::try_from_env("IRONBAR_LOG").or_else(|_| EnvFilter::try_new(DEFAULT_LOG))?;

    let file_filter_layer = EnvFilter::try_from_env("IRONBAR_FILE_LOG")
        .or_else(|_| EnvFilter::try_new(DEFAULT_FILE_LOG))?;

    let log_path = data_dir().unwrap_or(env::current_dir()?).join("ironbar");

    let appender = tracing_appender::rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("ironbar")
        .filename_suffix("log")
        .max_log_files(3)
        .build(log_path)?;

    let (file_writer, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .with(
            Layer::default()
                .with_writer(MakeFileWriter::new(file_writer))
                .with_ansi(false)
                .with_filter(file_filter_layer),
        )
        .init();

    Ok(guard)
}
