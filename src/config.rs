use crate::modules::clock::ClockModule;
use crate::modules::focused::FocusedModule;
use crate::modules::launcher::LauncherModule;
use crate::modules::mpd::MpdModule;
use crate::modules::script::ScriptModule;
use crate::modules::sysinfo::SysInfoModule;
use crate::modules::tray::TrayModule;
use crate::modules::workspaces::WorkspacesModule;
use color_eyre::eyre::{Context, ContextCompat};
use color_eyre::{eyre, Help, Report};
use dirs::config_dir;
use eyre::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ModuleConfig {
    Clock(ClockModule),
    Mpd(MpdModule),
    Tray(TrayModule),
    Workspaces(WorkspacesModule),
    SysInfo(SysInfoModule),
    Launcher(LauncherModule),
    Script(ScriptModule),
    Focused(FocusedModule),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MonitorConfig {
    Single(Config),
    Multiple(Vec<Config>),
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BarPosition {
    Top,
    Bottom,
}

impl Default for BarPosition {
    fn default() -> Self {
        Self::Bottom
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default = "default_bar_position")]
    pub position: BarPosition,
    #[serde(default = "default_bar_height")]
    pub height: i32,

    pub left: Option<Vec<ModuleConfig>>,
    pub center: Option<Vec<ModuleConfig>>,
    pub right: Option<Vec<ModuleConfig>>,

    pub monitors: Option<HashMap<String, MonitorConfig>>,
}

const fn default_bar_position() -> BarPosition {
    BarPosition::Bottom
}

const fn default_bar_height() -> i32 {
    42
}

impl Config {
    /// Attempts to load the config file from file,
    /// parse it and return a new instance of `Self`.
    pub fn load() -> Result<Self> {
        let config_path = if let Ok(config_path) = env::var("IRONBAR_CONFIG") {
            let path = PathBuf::from(config_path);
            if path.exists() {
                Ok(path)
            } else {
                Err(Report::msg("Specified config file does not exist")
                    .note("Config file was specified using `IRONBAR_CONFIG` environment variable"))
            }
        } else {
            Self::try_find_config()
        }?;

        Self::load_file(&config_path)
    }

    /// Attempts to discover the location of the config file
    /// by checking each valid format's extension.
    ///
    /// Returns the path of the first valid match, if any.
    fn try_find_config() -> Result<PathBuf> {
        let config_dir = config_dir().wrap_err("Failed to locate user config dir")?;

        let extensions = vec!["json", "toml", "yaml", "yml", "corn"];

        let file = extensions.into_iter().find_map(|extension| {
            let full_path = config_dir
                .join("ironbar")
                .join(format!("config.{extension}"));

            if Path::exists(&full_path) {
                Some(full_path)
            } else {
                None
            }
        });

        match file {
            Some(file) => Ok(file),
            None => Err(Report::msg("Could not find config file")),
        }
    }

    /// Loads the config file at the specified path
    /// and parses it into `Self` based on its extension.
    fn load_file(path: &Path) -> Result<Self> {
        let file = fs::read(path).wrap_err("Failed to read config file")?;
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        match extension {
            "json" => serde_json::from_slice(&file).wrap_err("Invalid JSON config"),
            "toml" => toml::from_slice(&file).wrap_err("Invalid TOML config"),
            "yaml" | "yml" => serde_yaml::from_slice(&file).wrap_err("Invalid YAML config"),
            "corn" => {
                // corn doesn't support deserialization yet
                // so serialize the interpreted result then deserialize that
                let file =
                    String::from_utf8(file).wrap_err("Config file contains invalid UTF-8")?;
                let config = cornfig::parse(&file).wrap_err("Invalid corn config")?.value;
                Ok(serde_json::from_str(&serde_json::to_string(&config)?)?)
            }
            _ => unreachable!(),
        }
    }
}

pub const fn default_false() -> bool {
    false
}
pub const fn default_true() -> bool {
    true
}
