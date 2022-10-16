use color_eyre::eyre::WrapErr;
use color_eyre::{Help, Report, Result};
use std::process::Command;
use tracing::instrument;

/// Attempts to execute a given command.
/// If the command returns status 0,
/// the `stdout` is returned.
/// Otherwise, an `Err` variant
/// containing the `stderr` is returned.
#[instrument]
pub fn exec_command(command: &str) -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .wrap_err("Failed to get script output")?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map(|output| output.trim().to_string())
            .wrap_err("Script stdout not valid UTF-8")?;

        Ok(stdout)
    } else {
        let stderr = String::from_utf8(output.stderr)
            .map(|output| output.trim().to_string())
            .wrap_err("Script stderr not valid UTF-8")?;

        Err(Report::msg(stderr)
            .wrap_err("Script returned non-zero error code")
            .suggestion("Check the path to your script")
            .suggestion("Check the script for errors"))
    }
}
