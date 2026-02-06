//! We are not using notify crate here, as /sys directly maps into the kernel
//! and the inotify support is limited and might depend on driver, event and kernel.

use std::{fmt, fs, path::PathBuf};

#[derive(Debug)]
pub enum ReadParseError {
    Read(std::io::Error),
    Parse(std::num::ParseIntError),
}

impl fmt::Display for ReadParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadParseError::Read(err) => write!(f, "I/O error: {}", err),
            ReadParseError::Parse(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl std::error::Error for ReadParseError {}

const SYS_PATH: &str = "/sys/class";

pub fn brightness(subsystem: &str, name: &str) -> Result<i32, ReadParseError> {
    let mut path = PathBuf::from(SYS_PATH);
    path.push(subsystem);
    path.push(name);
    path.push("brightness");
    let s = fs::read_to_string(path).map_err(ReadParseError::Read)?;
    let value = s.trim().parse::<i32>().map_err(ReadParseError::Parse)?;
    Ok(value)
}

pub fn max_brightness(subsystem: &str, name: &str) -> Result<i32, ReadParseError> {
    let mut path = PathBuf::from(SYS_PATH);
    path.push(subsystem);
    path.push(name);
    path.push("max_brightness");
    let s = fs::read_to_string(path).map_err(ReadParseError::Read)?;
    let value = s.trim().parse::<i32>().map_err(ReadParseError::Parse)?;
    Ok(value)
}

pub(super) fn available_resource_names(subsystem: &str) -> Vec<String> {
    let mut path = PathBuf::from(SYS_PATH);
    path.push(subsystem);

    let mut resource_names = Vec::new();
    if let Ok(entries) = fs::read_dir(&path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                resource_names.push(name.to_string());
            }
        }
    }
    resource_names.sort();
    resource_names
}

pub fn default_resource_name(subsystem: &str) -> Option<String> {
    let possible_files = available_resource_names(subsystem);

    match subsystem {
        "backlight" => {
            // harded list of common names
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
                .find(|item| possible_files.iter().any(|v| v == **item))
                .map(|s| s.to_string())
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
