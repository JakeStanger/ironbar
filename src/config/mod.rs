mod r#impl;
mod truncate;

#[cfg(feature = "clipboard")]
use crate::modules::clipboard::ClipboardModule;
#[cfg(feature = "clock")]
use crate::modules::clock::ClockModule;
use crate::modules::custom::CustomModule;
use crate::modules::focused::FocusedModule;
use crate::modules::launcher::LauncherModule;
#[cfg(feature = "music")]
use crate::modules::music::MusicModule;
use crate::modules::script::ScriptModule;
#[cfg(feature = "sys_info")]
use crate::modules::sysinfo::SysInfoModule;
#[cfg(feature = "tray")]
use crate::modules::tray::TrayModule;
#[cfg(feature = "workspaces")]
use crate::modules::workspaces::WorkspacesModule;
use crate::script::ScriptInput;
use serde::Deserialize;
use std::collections::HashMap;

pub use self::truncate::{EllipsizeMode, TruncateMode};

#[derive(Debug, Deserialize, Clone)]
pub struct CommonConfig {
    pub show_if: Option<ScriptInput>,

    pub on_click_left: Option<ScriptInput>,
    pub on_click_right: Option<ScriptInput>,
    pub on_click_middle: Option<ScriptInput>,
    pub on_scroll_up: Option<ScriptInput>,
    pub on_scroll_down: Option<ScriptInput>,

    pub tooltip: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModuleConfig {
    #[cfg(feature = "clock")]
    Clipboard(Box<ClipboardModule>),
    #[cfg(feature = "clock")]
    Clock(Box<ClockModule>),
    Custom(Box<CustomModule>),
    Focused(Box<FocusedModule>),
    Launcher(Box<LauncherModule>),
    #[cfg(feature = "music")]
    Music(Box<MusicModule>),
    Script(Box<ScriptModule>),
    #[cfg(feature = "sys_info")]
    SysInfo(Box<SysInfoModule>),
    #[cfg(feature = "tray")]
    Tray(Box<TrayModule>),
    #[cfg(feature = "workspaces")]
    Workspaces(Box<WorkspacesModule>),
}

#[derive(Debug, Clone)]
pub enum MonitorConfig {
    Single(Config),
    Multiple(Vec<Config>),
}

#[derive(Debug, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Default, Deserialize, Copy, Clone, PartialEq, Eq)]
pub struct MarginConfig {
    #[serde(default)]
    pub bottom: i32,
    #[serde(default)]
    pub left: i32,
    #[serde(default)]
    pub right: i32,
    #[serde(default)]
    pub top: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub position: BarPosition,
    #[serde(default = "default_true")]
    pub anchor_to_edges: bool,
    #[serde(default = "default_bar_height")]
    pub height: i32,
    #[serde(default)]
    pub margin: MarginConfig,

    /// GTK icon theme to use.
    pub icon_theme: Option<String>,

    pub start: Option<Vec<ModuleConfig>>,
    pub center: Option<Vec<ModuleConfig>>,
    pub end: Option<Vec<ModuleConfig>>,

    pub monitors: Option<HashMap<String, MonitorConfig>>,
}

const fn default_bar_height() -> i32 {
    42
}

pub const fn default_false() -> bool {
    false
}

pub const fn default_true() -> bool {
    true
}
