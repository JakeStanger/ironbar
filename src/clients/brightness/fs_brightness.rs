use std::{fmt, fs, path::PathBuf};

#[derive(Debug, Default)]
pub struct FsLogin1Session {}

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

impl FsLogin1Session {
    pub fn brightness(&self, subsystem: &str, name: &str) -> Result<i32, ReadParseError> {
        let mut path = PathBuf::from(SYS_PATH);
        path.push(subsystem);
        path.push(name);
        path.push("brightness");
        let s = fs::read_to_string(path).map_err(ReadParseError::Read)?;
        let value = s.trim().parse::<i32>().map_err(ReadParseError::Parse)?;
        Ok(value)
    }

    pub fn max_brightness(&self, subsystem: &str, name: &str) -> Result<i32, ReadParseError> {
        let mut path = PathBuf::from(SYS_PATH);
        path.push(subsystem);
        path.push(name);
        path.push("max_brightness");
        let s = fs::read_to_string(path).map_err(ReadParseError::Read)?;
        let value = s.trim().parse::<i32>().map_err(ReadParseError::Parse)?;
        Ok(value)
    }
}

pub fn default_resource_name(subsystem: &str) -> Option<String> {
    let mut path = PathBuf::from(SYS_PATH);
    path.push(subsystem);

    let mut possible_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                possible_files.push(name.to_string());
            }
        }
    }
    possible_files.sort();
    let possible_files = possible_files;

    match subsystem {
        "backlight" => {
            // harded list of common names
            let to_check = [
                "amdgpu_bl0",
                "amdgpu_bl1",
                "intel_backlight",
                "radeon_bl",
                "nvidia_0",
                "nvidia_1",
                "nouveau_backlight",
                "acpi_video0",
                "acpi_video1",
            ]
            .into_iter();
            to_check
                .filter_map(|item| possible_files.iter().find(|v| v.as_str() == item))
                .next()
                .cloned()
                .or_else(|| possible_files.first().cloned())
        }
        "leds" => {
            // almost all leds have a specific postfix
            let common_postfix = ["::kdb_backlight"].into_iter();
            common_postfix
                .filter_map(|item| possible_files.iter().find(|v| v.ends_with(item)))
                .next()
                .cloned()
                .or_else(|| possible_files.first().cloned())
        }
        _ => {
            // return first directory
            possible_files.first().cloned()
        }
    }
}
