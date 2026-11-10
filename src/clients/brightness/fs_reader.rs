//! We are not using notify crate here, as /sys directly maps into the kernel
//! and the inotify support is limited and might depend on driver, event and kernel.

use std::{fmt, fs, path::PathBuf};

#[derive(Debug)]
pub enum ReadParseError {
    Read(std::io::Error),
    Parse(std::num::ParseIntError),
}

impl From<std::io::Error> for ReadParseError {
    fn from(value: std::io::Error) -> Self {
        Self::Read(value)
    }
}

impl From<std::num::ParseIntError> for ReadParseError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::Parse(value)
    }
}

impl fmt::Display for ReadParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadParseError::Read(err) => write!(f, "I/O error: {err}"),
            ReadParseError::Parse(err) => write!(f, "Parse error: {err}"),
        }
    }
}

impl std::error::Error for ReadParseError {}

const SYS_PATH: &str = "/sys/class";

/// Build path from `subsystem` and `name`.
///
/// The path built has [SYS_PATH] as a prefix.
fn build_path(subsystem: &str, name: &str) -> PathBuf {
    let mut path = PathBuf::from(SYS_PATH);
    path.push(subsystem);
    path.push(name);
    path
}

/// Read an [i32] for a specific `attribute`.
///
/// Select within [SYS_PATH] the `subsystem` and the `driver` for the `attribute` you want to read an [i32] from.
fn read_i32_from_file(
    subsystem: &str,
    driver: &str,
    attribute: &str,
) -> Result<i32, ReadParseError> {
    let mut path = build_path(subsystem, driver);
    path.push(attribute);
    let s = fs::read_to_string(path)?;
    Ok(s.trim().parse::<i32>()?)
}

/// Get the `brightness` attribute from the `subsystem` and `driver` within [SYS_PATH].
pub fn brightness(subsystem: &str, driver: &str) -> Result<i32, ReadParseError> {
    read_i32_from_file(subsystem, driver, "brightness")
}

/// Get the `max_brightness` attribute from the `subsystem` and `driver` within [SYS_PATH].
pub fn max_brightness(subsystem: &str, driver: &str) -> Result<i32, ReadParseError> {
    read_i32_from_file(subsystem, driver, "max_brightness")
}

/// Get the scaling of the brightness.
///
/// Possible values are:
///     - "unknown"
///     - "linear"
pub(super) fn scaling(subsystem: &str, driver: &str) -> Result<Box<str>, ReadParseError> {
    let mut path = build_path(subsystem, driver);
    path.push("scale");
    Ok(Box::from(fs::read_to_string(path)?.as_str()))
}

pub(super) fn available_resource_names(subsystem: &str) -> Vec<Box<str>> {
    let mut path = PathBuf::from(SYS_PATH);
    path.push(subsystem);

    let mut resource_names = Vec::new();
    if let Ok(entries) = fs::read_dir(&path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                resource_names.push(Box::from(name));
            }
        }
    }
    resource_names.sort();
    resource_names
}

pub fn default_resource_name(subsystem: &str) -> Option<Box<str>> {
    let possible_files = available_resource_names(subsystem);

    match subsystem {
        "backlight" => {
            // list of common names
            const TO_CHECK: [&str; 10] = [
                "amdgpu_bl0",
                "amdgpu_bl1",
                "intel_backlight",
                "radeon_bl",
                "nvidia_0",
                "nvidia_1",
                "nouveau_backlight",
                "acpi_video0",
                "acpi_video1",
                "apple-panel-bl",
            ];

            TO_CHECK
                .iter()
                .find(|&item| possible_files.iter().any(|v| **v == **item))
                .map(|s| Box::from(*s))
        }
        "leds" => {
            // almost all leds have the same postfix
            possible_files
                .into_iter()
                .find(|v| v.ends_with("kdb_backlight"))
        }
        _ => None,
    }
}
