use crate::modules::clock::ClockModule;
use crate::modules::focused::FocusedModule;
use crate::modules::launcher::LauncherModule;
use crate::modules::mpd::MpdModule;
use crate::modules::script::ScriptModule;
use crate::modules::sysinfo::SysInfoModule;
use crate::modules::tray::TrayModule;
use crate::modules::workspaces::WorkspacesModule;
use dirs::config_dir;
use serde::Deserialize;
use std::fs;

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

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BarPosition {
    Top,
    Bottom,
}

impl Default for BarPosition {
    fn default() -> Self {
        BarPosition::Bottom
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

    pub monitors: Option<Vec<Config>>,
}

const fn default_bar_position() -> BarPosition {
    BarPosition::Bottom
}

const fn default_bar_height() -> i32 {
    42
}

impl Config {
    pub fn load() -> Option<Self> {
        let config_dir = config_dir().expect("Failed to locate user config dir");

        let extensions = vec!["json", "toml", "yaml", "yml", "corn"];

        extensions.into_iter().find_map(|extension| {
            let full_path = config_dir
                .join("ironbar")
                .join(format!("config.{extension}"));

            if full_path.exists() {
                let file = fs::read(full_path).expect("Failed to read config file");
                Some(match extension {
                    "json" => serde_json::from_slice(&file).expect("Invalid JSON config"),
                    "toml" => toml::from_slice(&file).expect("Invalid TOML config"),
                    "yaml" | "yml" => serde_yaml::from_slice(&file).expect("Invalid YAML config"),
                    "corn" => {
                        // corn doesn't support deserialization yet
                        // so serialize the interpreted result then deserialize that
                        let file =
                            String::from_utf8(file).expect("Config file contains invalid UTF-8");
                        let config = cornfig::parse(&file).expect("Invalid corn config").value;
                        serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap()
                    }
                    _ => unreachable!(),
                })
            } else {
                None
            }
        })
    }
}

pub const fn default_false() -> bool {
    false
}
pub const fn default_true() -> bool {
    true
}
