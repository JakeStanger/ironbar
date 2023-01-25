mod r#impl;

use crate::modules::clock::ClockModule;
use crate::modules::custom::CustomModule;
use crate::modules::focused::FocusedModule;
use crate::modules::launcher::LauncherModule;
use crate::modules::music::MusicModule;
use crate::modules::script::ScriptModule;
use crate::modules::sysinfo::SysInfoModule;
use crate::modules::tray::TrayModule;
use crate::modules::workspaces::WorkspacesModule;
use crate::script::ScriptInput;
use serde::Deserialize;
use std::collections::HashMap;

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
    Clock(ClockModule),
    Music(MusicModule),
    Tray(TrayModule),
    Workspaces(WorkspacesModule),
    SysInfo(SysInfoModule),
    Launcher(LauncherModule),
    Script(ScriptModule),
    Focused(FocusedModule),
    Custom(CustomModule),
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

pub const fn default_false() -> bool {
    false
}

pub const fn default_true() -> bool {
    true
}
