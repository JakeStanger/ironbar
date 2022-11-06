use crate::modules::clock::ClockModule;
use crate::modules::custom::CustomModule;
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
use gtk::Orientation;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use tracing::instrument;

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
    Custom(CustomModule),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MonitorConfig {
    Single(Config),
    Multiple(Vec<Config>),
}

#[derive(Debug, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BarPosition {
    Top,
    Bottom,
    Left,
    Right,
}

impl Default for BarPosition {
    fn default() -> Self {
        Self::Bottom
    }
}

impl BarPosition {
    pub fn get_orientation(self) -> Orientation {
        if self == Self::Top || self == Self::Bottom {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        }
    }

    pub const fn get_angle(self) -> f64 {
        match self {
            Self::Top | Self::Bottom => 0.0,
            Self::Left => 90.0,
            Self::Right => 270.0,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_bar_position")]
    pub position: BarPosition,
    #[serde(default = "default_true")]
    pub anchor_to_edges: bool,
    #[serde(default = "default_bar_height")]
    pub height: i32,

    pub start: Option<Vec<ModuleConfig>>,
    pub center: Option<Vec<ModuleConfig>>,
    pub end: Option<Vec<ModuleConfig>>,

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
    #[instrument]
    pub fn load() -> Result<Self> {
        let config_path = env::var("IRONBAR_CONFIG").map_or_else(
            |_| Self::try_find_config(),
            |config_path| {
                let path = PathBuf::from(config_path);
                if path.exists() {
                    Ok(path)
                } else {
                    Err(Report::msg(format!(
                        "Specified config file does not exist: {}",
                        path.display()
                    ))
                    .note("Config file was specified using `IRONBAR_CONFIG` environment variable"))
                }
            },
        )?;

        Self::load_file(&config_path)
    }

    /// Attempts to discover the location of the config file
    /// by checking each valid format's extension.
    ///
    /// Returns the path of the first valid match, if any.
    #[instrument]
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

        file.map_or_else(
            || {
                Err(Report::msg("Could not find config file")
                    .suggestion("Ironbar does not include a configuration out of the box")
                    .suggestion("A guide on writing a config can be found on the wiki:")
                    .suggestion("https://github.com/JakeStanger/ironbar/wiki/configuration-guide"))
            },
            Ok,
        )
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
                let config = libcorn::parse(&file).wrap_err("Invalid corn config")?.value;
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
