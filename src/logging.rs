use color_eyre::Result;
use dirs::data_dir;
use std::env;
use strip_ansi_escapes::Writer;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
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

pub fn install_tracing() -> Result<WorkerGuard> {
    let fmt_layer = fmt::layer().with_target(true);
    let filter_layer = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;
    let file_filter_layer =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("warn"))?;

    let log_path = data_dir().unwrap_or(env::current_dir()?).join("ironbar");

    let appender = tracing_appender::rolling::never(log_path, "error.log");
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
