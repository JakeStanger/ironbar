mod common;
mod r#impl;
mod layout;
mod truncate;

#[cfg(feature = "cairo")]
use crate::modules::cairo::CairoModule;
#[cfg(feature = "clipboard")]
use crate::modules::clipboard::ClipboardModule;
#[cfg(feature = "clock")]
use crate::modules::clock::ClockModule;
#[cfg(feature = "custom")]
use crate::modules::custom::CustomModule;
#[cfg(feature = "focused")]
use crate::modules::focused::FocusedModule;
#[cfg(feature = "keyboard")]
use crate::modules::keyboard::KeyboardModule;
#[cfg(feature = "label")]
use crate::modules::label::LabelModule;
#[cfg(feature = "launcher")]
use crate::modules::launcher::LauncherModule;
#[cfg(feature = "music")]
use crate::modules::music::MusicModule;
#[cfg(feature = "network_manager")]
use crate::modules::networkmanager::NetworkManagerModule;
#[cfg(feature = "notifications")]
use crate::modules::notifications::NotificationsModule;
#[cfg(feature = "script")]
use crate::modules::script::ScriptModule;
#[cfg(feature = "sway")]
use crate::modules::sway::mode::SwayModeModule;
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

use crate::modules::{AnyModuleFactory, ModuleFactory, ModuleInfo};
use cfg_if::cfg_if;
use color_eyre::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[cfg(feature = "schema")]
use schemars::JsonSchema;

pub use self::common::{CommonConfig, ModuleJustification, ModuleOrientation, TransitionType};
pub use self::layout::LayoutConfig;
pub use self::truncate::{EllipsizeMode, TruncateMode};

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ModuleConfig {
    #[cfg(feature = "cairo")]
    Cairo(Box<CairoModule>),
    #[cfg(feature = "clipboard")]
    Clipboard(Box<ClipboardModule>),
    #[cfg(feature = "clock")]
    Clock(Box<ClockModule>),
    #[cfg(feature = "custom")]
    Custom(Box<CustomModule>),
    #[cfg(feature = "focused")]
    Focused(Box<FocusedModule>),
    #[cfg(feature = "keyboard")]
    Keyboard(Box<KeyboardModule>),
    #[cfg(feature = "label")]
    Label(Box<LabelModule>),
    #[cfg(feature = "launcher")]
    Launcher(Box<LauncherModule>),
    #[cfg(feature = "music")]
    Music(Box<MusicModule>),
    #[cfg(feature = "network_manager")]
    NetworkManager(Box<NetworkManagerModule>),
    #[cfg(feature = "notifications")]
    Notifications(Box<NotificationsModule>),
    #[cfg(feature = "script")]
    Script(Box<ScriptModule>),
    #[cfg(feature = "sys_info")]
    SysInfo(Box<SysInfoModule>),
    #[cfg(feature = "sway")]
    SwayMode(Box<SwayModeModule>),
    #[cfg(feature = "tray")]
    Tray(Box<TrayModule>),
    #[cfg(feature = "upower")]
    Upower(Box<UpowerModule>),
    #[cfg(feature = "volume")]
    Volume(Box<VolumeModule>),
    #[cfg(feature = "workspaces")]
    Workspaces(Box<WorkspacesModule>),
}

impl ModuleConfig {
    pub fn create(
        self,
        module_factory: &AnyModuleFactory,
        container: &gtk::Box,
        info: &ModuleInfo,
    ) -> Result<()> {
        macro_rules! create {
            ($module:expr) => {
                module_factory.create(*$module, container, info)
            };
        }

        match self {
            #[cfg(feature = "cairo")]
            Self::Cairo(module) => create!(module),
            #[cfg(feature = "clipboard")]
            Self::Clipboard(module) => create!(module),
            #[cfg(feature = "clock")]
            Self::Clock(module) => create!(module),
            #[cfg(feature = "custom")]
            Self::Custom(module) => create!(module),
            #[cfg(feature = "focused")]
            Self::Focused(module) => create!(module),
            #[cfg(feature = "keyboard")]
            Self::Keyboard(module) => create!(module),
            #[cfg(feature = "label")]
            Self::Label(module) => create!(module),
            #[cfg(feature = "launcher")]
            Self::Launcher(module) => create!(module),
            #[cfg(feature = "music")]
            Self::Music(module) => create!(module),
            #[cfg(feature = "network_manager")]
            Self::NetworkManager(module) => create!(module),
            #[cfg(feature = "notifications")]
            Self::Notifications(module) => create!(module),
            #[cfg(feature = "script")]
            Self::Script(module) => create!(module),
            #[cfg(feature = "sys_info")]
            Self::SysInfo(module) => create!(module),
            #[cfg(feature = "sway")]
            Self::SwayMode(module) => create!(module),
            #[cfg(feature = "tray")]
            Self::Tray(module) => create!(module),
            #[cfg(feature = "upower")]
            Self::Upower(module) => create!(module),
            #[cfg(feature = "volume")]
            Self::Volume(module) => create!(module),
            #[cfg(feature = "workspaces")]
            Self::Workspaces(module) => create!(module),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum MonitorConfig {
    Single(BarConfig),
    Multiple(Vec<BarConfig>),
}

#[derive(Debug, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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

/// The following is a list of all top-level bar config options.
///
/// These options can either be written at the very top object of your config,
/// or within an object in the [monitors](#monitors) config,
/// depending on your [use-case](#2-pick-your-use-case).
///
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct BarConfig {
    /// A unique identifier for the bar, used for controlling it over IPC.
    /// If not set, uses a generated integer suffix.
    ///
    /// **Default**: `bar-n`
    pub name: Option<String>,

    /// The bar's position on screen.
    ///
    /// **Valid options**: `top`, `bottom`, `left`, `right`
    /// <br>
    /// **Default**: `bottom`
    #[serde(default)]
    pub position: BarPosition,

    /// Whether to anchor the bar to the edges of the screen.
    /// Setting to false centers the bar.
    ///
    /// **Default**: `true`
    #[serde(default = "default_true")]
    pub anchor_to_edges: bool,

    /// The bar's height in pixels.
    ///
    /// Note that GTK treats this as a target minimum,
    /// and if content inside the bar is over this,
    /// it will automatically expand to fit.
    ///
    /// **Default**: `42`
    #[serde(default = "default_bar_height")]
    pub height: i32,

    /// The margin to use on each side of the bar, in pixels.
    /// Object which takes `top`, `bottom`, `left` and `right` keys.
    ///
    /// **Default**: `0` on all sides.
    ///
    /// # Example
    ///
    /// The following would set a 10px margin around each edge.
    ///
    /// ```corn
    /// {
    ///     margin.top = 10
    ///     margin.bottom = 10
    ///     margin.left = 10
    ///     margin.right = 10
    /// }
    /// ```
    #[serde(default)]
    pub margin: MarginConfig,

    /// The layer-shell layer to place the bar on.
    ///
    /// Taken from the
    /// [wlr_layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_shell_v1:enum:layer) definition:
    ///
    /// > These values indicate which layers a surface can be rendered in.
    /// > They are ordered by z depth, bottom-most first.
    /// > Traditional shell surfaces will typically be rendered between the bottom and top layers.
    /// > Fullscreen shell surfaces are typically rendered at the top layer.
    /// > Multiple surfaces can share a single layer, and ordering within a single layer is undefined.
    ///
    /// **Valid options**: `background`, `bottom`, `top`, `overlay`
    /// <br>
    /// **Default**: `top`
    #[serde(
        default = "default_layer",
        deserialize_with = "r#impl::deserialize_layer"
    )]
    #[cfg_attr(feature = "schema", schemars(schema_with = "r#impl::schema_layer"))]
    pub layer: gtk_layer_shell::Layer,

    /// Whether the bar should reserve an exclusive zone around it.
    ///
    /// When true, this prevents windows from rendering in the same space
    /// as the bar, causing them to shift.
    ///
    /// **Default**: `true` unless `start_hidden` is set.
    #[serde(default)]
    pub exclusive_zone: Option<bool>,

    /// The size of the gap in pixels
    /// between the bar and the popup window.
    ///
    /// **Default**: `5`
    #[serde(default = "default_popup_gap")]
    pub popup_gap: i32,

    /// Whether the bar should be hidden when Ironbar starts.
    ///
    /// **Default**: `false`, unless `autohide` is set.
    #[serde(default)]
    pub start_hidden: Option<bool>,

    /// The duration in milliseconds before the bar is hidden after the cursor leaves.
    /// Leave unset to disable auto-hide behaviour.
    ///
    /// **Default**: `null`
    #[serde(default)]
    pub autohide: Option<u64>,

    // /// The name of the GTK icon theme to use.
    // /// Leave unset to use the default Adwaita theme.
    // ///
    // /// **Default**: `null`
    // pub icon_theme: Option<String>,
    /// An array of modules to append to the start of the bar.
    /// Depending on the orientation, this is either the top of the left edge.
    ///
    /// **Default**: `[]`
    pub start: Option<Vec<ModuleConfig>>,

    /// An array of modules to append to the center of the bar.
    ///
    /// **Default**: `[]`
    pub center: Option<Vec<ModuleConfig>>,

    /// An array of modules to append to the end of the bar.
    /// Depending on the orientation, this is either the bottom or right edge.
    ///
    /// **Default**: `[]`
    pub end: Option<Vec<ModuleConfig>>,
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
            margin: MarginConfig::default(),
            name: None,
            layer: default_layer(),
            exclusive_zone: None,
            height: default_bar_height(),
            start_hidden: None,
            autohide: None,
            // icon_theme: None,
            #[cfg(feature = "label")]
            start: Some(vec![ModuleConfig::Label(
                LabelModule::new("ℹ️ Using default config".to_string()).into(),
            )]),
            #[cfg(not(feature = "label"))]
            start: None,
            center,
            end,
            anchor_to_edges: default_true(),
            popup_gap: default_popup_gap(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Config {
    /// A map of [ironvar](ironvar) keys and values
    /// to initialize Ironbar with on startup.
    ///
    /// **Default**: `{}`
    ///
    /// # Example
    ///
    /// The following initializes an ironvar called `foo` set to `bar` on startup:
    ///
    /// ```corn
    /// { ironvar_defaults.foo = "bar" }
    /// ```
    ///
    /// The variable can then be immediately fetched without needing to be manually set:
    ///
    /// ```sh
    /// $ ironbar get foo
    /// ok
    /// bar
    /// ```
    pub ironvar_defaults: Option<HashMap<Box<str>, String>>,

    /// The configuration for the bar.
    /// Setting through this will enable a single identical bar on each monitor.
    #[serde(flatten)]
    pub bar: BarConfig,

    /// A map of monitor names to configs.
    ///
    /// The config values can be either:
    ///
    /// - a single object, which denotes a single bar for that monitor,
    /// - an array of multiple objects, which denotes multiple for that monitor.
    ///
    /// Providing this option overrides the single, global `bar` option.
    pub monitors: Option<HashMap<String, MonitorConfig>>,

    /// Map of app IDs (or classes) to icon names,
    /// overriding the app's default icon.
    ///
    /// **Default**: `{}`
    #[serde(default)]
    pub icon_overrides: HashMap<String, String>,
}

const fn default_layer() -> gtk_layer_shell::Layer {
    gtk_layer_shell::Layer::Top
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
