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

impl FsLogin1Session {
    pub fn get_brightness(&self, subsystem: &str, name: &str) -> Result<i32, ReadParseError> {
        let mut path = PathBuf::from("/sys/class");
        path.push(subsystem);
        path.push(name);
        path.push("brightness");
        let s = fs::read_to_string(path).map_err(ReadParseError::Read)?;
        let value = s.trim().parse::<i32>().map_err(ReadParseError::Parse)?;
        Ok(value)
    }

    pub fn get_max_brightness(&self, subsystem: &str, name: &str) -> Result<i32, ReadParseError> {
        let mut path = PathBuf::from("/sys/class");
        path.push(subsystem);
        path.push(name);
        path.push("max_brightness");
        let s = fs::read_to_string(path).map_err(ReadParseError::Read)?;
        let value = s.trim().parse::<i32>().map_err(ReadParseError::Parse)?;
        Ok(value)
    }
}
