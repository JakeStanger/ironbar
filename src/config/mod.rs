mod common;
mod r#impl;
mod truncate;

#[cfg(feature = "clipboard")]
use crate::modules::clipboard::ClipboardModule;
#[cfg(feature = "clock")]
use crate::modules::clock::ClockModule;
use crate::modules::custom::CustomModule;
#[cfg(feature = "focused")]
use crate::modules::focused::FocusedModule;
use crate::modules::label::LabelModule;
#[cfg(feature = "launcher")]
use crate::modules::launcher::LauncherModule;
#[cfg(feature = "music")]
use crate::modules::music::MusicModule;
#[cfg(feature = "networkmanager")]
use crate::modules::networkmanager::NetworkManagerModule;
#[cfg(feature = "notifications")]
use crate::modules::notifications::NotificationsModule;
use crate::modules::script::ScriptModule;
#[cfg(feature = "sys_info")]
use crate::modules::sysinfo::SysInfoModule;
#[cfg(feature = "tray")]
use crate::modules::tray::TrayModule;
#[cfg(feature = "upower")]
use crate::modules::upower::UpowerModule;
#[cfg(feature = "volume")]
use crate::modules::volume::VolumeModule;
#[cfg(feature = "workspaces")]
use crate::modules::workspaces::WorkspacesModule;
use cfg_if::cfg_if;
use serde::Deserialize;
use std::collections::HashMap;

pub use self::common::{CommonConfig, TransitionType};
pub use self::truncate::TruncateMode;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModuleConfig {
    #[cfg(feature = "clipboard")]
    Clipboard(Box<ClipboardModule>),
    #[cfg(feature = "clock")]
    Clock(Box<ClockModule>),
    Custom(Box<CustomModule>),
    #[cfg(feature = "focused")]
    Focused(Box<FocusedModule>),
    Label(Box<LabelModule>),
    #[cfg(feature = "launcher")]
    Launcher(Box<LauncherModule>),
    #[cfg(feature = "music")]
    Music(Box<MusicModule>),
    #[cfg(feature = "networkmanager")]
    Networkmanager(Box<NetworkManagerModule>),
    #[cfg(feature = "notifications")]
    Notifications(Box<NotificationsModule>),
    Script(Box<ScriptModule>),
    #[cfg(feature = "sys_info")]
    SysInfo(Box<SysInfoModule>),
    #[cfg(feature = "tray")]
    Tray(Box<TrayModule>),
    #[cfg(feature = "upower")]
    Upower(Box<UpowerModule>),
    #[cfg(feature = "volume")]
    Volume(Box<VolumeModule>),
    #[cfg(feature = "workspaces")]
    Workspaces(Box<WorkspacesModule>),
}

#[derive(Debug, Deserialize, Clone)]
pub enum BarEntryConfig {
    Single(BarConfig),
    Monitors(HashMap<String, MonitorConfig>),
}

#[derive(Debug, Clone)]
pub enum MonitorConfig {
    Single(BarConfig),
    Multiple(Vec<BarConfig>),
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
pub struct BarConfig {
    #[serde(default)]
    pub position: BarPosition,
    #[serde(default = "default_true")]
    pub anchor_to_edges: bool,
    #[serde(default = "default_bar_height")]
    pub height: i32,
    #[serde(default)]
    pub margin: MarginConfig,
    pub name: Option<String>,

    #[serde(default)]
    pub start_hidden: Option<bool>,
    #[serde(default)]
    pub autohide: Option<u64>,

    /// GTK icon theme to use.
    pub icon_theme: Option<String>,

    pub start: Option<Vec<ModuleConfig>>,
    pub center: Option<Vec<ModuleConfig>>,
    pub end: Option<Vec<ModuleConfig>>,

    #[serde(default = "default_popup_gap")]
    pub popup_gap: i32,
}

impl Default for BarConfig {
    fn default() -> Self {
        cfg_if! {
            if #[cfg(feature = "clock")] {
                let end = Some(vec![ModuleConfig::Clock(Box::default())]);
            }
            else {
                let end = None;
            }
        }

        cfg_if! {
            if #[cfg(feature = "focused")] {
                let center = Some(vec![ModuleConfig::Focused(Box::default())]);
            }
            else {
                let center = None;
            }
        }

        Self {
            position: BarPosition::default(),
            height: default_bar_height(),
            margin: MarginConfig::default(),
            name: None,
            start_hidden: None,
            autohide: None,
            icon_theme: None,
            start: Some(vec![ModuleConfig::Label(
                LabelModule::new("ℹ️ Using default config".to_string()).into(),
            )]),
            center,
            end,
            anchor_to_edges: default_true(),
            popup_gap: default_popup_gap(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub ironvar_defaults: Option<HashMap<Box<str>, String>>,

    #[serde(flatten)]
    pub bar: BarConfig,
    pub monitors: Option<HashMap<String, MonitorConfig>>,
}

const fn default_bar_height() -> i32 {
    42
}

const fn default_popup_gap() -> i32 {
    5
}

pub const fn default_false() -> bool {
    false
}

pub const fn default_true() -> bool {
    true
}
