use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum ConfigLocation {
    Minimal,
    Desktop,
    Custom(PathBuf),
}

impl FromStr for ConfigLocation {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "minimal" => Ok(ConfigLocation::Minimal),
            "desktop" => Ok(ConfigLocation::Desktop),
            _ => Ok(ConfigLocation::Custom(PathBuf::from(s))),
        }
    }
}
