mod common;
pub mod default;
mod r#impl;
mod layout;
mod marquee;
mod truncate;

#[cfg(feature = "battery")]
use crate::modules::battery::BatteryModule;
#[cfg(feature = "bindmode")]
use crate::modules::bindmode::Bindmode;
#[cfg(feature = "bluetooth")]
use crate::modules::bluetooth::BluetoothModule;
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
#[cfg(feature = "inhibit")]
use crate::modules::inhibit::InhibitModule;
#[cfg(feature = "keyboard")]
use crate::modules::keyboard::KeyboardModule;
#[cfg(feature = "label")]
use crate::modules::label::LabelModule;
#[cfg(feature = "launcher")]
use crate::modules::launcher::LauncherModule;
#[cfg(feature = "menu")]
use crate::modules::menu::MenuModule;
#[cfg(feature = "music")]
use crate::modules::music::MusicModule;
#[cfg(feature = "network_manager")]
use crate::modules::networkmanager::NetworkManagerModule;
#[cfg(feature = "notifications")]
use crate::modules::notifications::NotificationsModule;
#[cfg(feature = "script")]
use crate::modules::script::ScriptModule;
#[cfg(feature = "sys_info")]
use crate::modules::sysinfo::SysInfoModule;
#[cfg(feature = "tray")]
use crate::modules::tray::TrayModule;
#[cfg(feature = "volume")]
use crate::modules::volume::VolumeModule;
#[cfg(feature = "workspaces")]
use crate::modules::workspaces::WorkspacesModule;

pub use self::common::{CommonConfig, ModuleJustification, ModuleOrientation, TransitionType};
pub use self::layout::LayoutConfig;
pub use self::marquee::{MarqueeMode, MarqueeOnHover};
pub use self::truncate::{EllipsizeMode, TruncateMode};

use gtk::prelude::ObjectExt;
use std::sync::OnceLock;

/// Global double-click time setting
static DOUBLE_CLICK_TIME: OnceLock<DoubleClickTime> = OnceLock::new();

/// Track if we've set GTK's setting yet
static GTK_SETTING_INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initialize the global double-click time setting
pub fn set_double_click_time(time: DoubleClickTime) {
    let _ = DOUBLE_CLICK_TIME.set(time);
}

/// Get the configured double-click time in milliseconds
pub fn get_double_click_time_ms() -> u64 {
    // Initialize GTK's setting once (after GTK is initialized)
    GTK_SETTING_INITIALIZED.get_or_init(|| {
        if let (Some(DoubleClickTime::Ms(ms)), Some(settings)) =
            (DOUBLE_CLICK_TIME.get(), gtk::Settings::default())
        {
            settings.set_property("gtk-double-click-time", *ms as i32);
        }
    });

    DOUBLE_CLICK_TIME
        .get()
        .map(|t| match t {
            DoubleClickTime::Ms(ms) => *ms,
            DoubleClickTime::Gtk => {
                // Read from GTK settings
                gtk::Settings::default()
                    .map(|settings| settings.property::<i32>("gtk-double-click-time") as u64)
                    .unwrap_or(400) // GTK default fallback
            }
        })
        .expect("double_click_time should be initialized during config load")
}
use crate::Ironbar;
use crate::modules::{AnyModuleFactory, ModuleFactory, ModuleInfo, ModuleRef};
use crate::style::CssSource;
use cfg_if::cfg_if;
use color_eyre::Result;
use config::FileFormat;
#[cfg(feature = "extras")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::{error, warn};

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(JsonSchema))]
pub enum ModuleConfig {
    #[cfg(feature = "battery")]
    Battery(Box<BatteryModule>),
    #[cfg(feature = "bindmode")]
    Bindmode(Box<Bindmode>),
    #[cfg(feature = "bluetooth")]
    Bluetooth(Box<BluetoothModule>),
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
    #[cfg(feature = "inhibit")]
    Inhibit(Box<InhibitModule>),
    #[cfg(feature = "keyboard")]
    Keyboard(Box<KeyboardModule>),
    #[cfg(feature = "label")]
    Label(Box<LabelModule>),
    #[cfg(feature = "launcher")]
    Launcher(Box<LauncherModule>),
    #[cfg(feature = "menu")]
    Menu(Box<MenuModule>),
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
    #[cfg(feature = "tray")]
    Tray(Box<TrayModule>),
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
    ) -> Result<ModuleRef> {
        macro_rules! create {
            ($module:expr) => {
                module_factory.create(*$module, container, info)
            };
        }

        match self {
            #[cfg(feature = "battery")]
            Self::Battery(module) => create!(module),
            #[cfg(feature = "bindmode")]
            Self::Bindmode(module) => create!(module),
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth(module) => create!(module),
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
            #[cfg(feature = "inhibit")]
            Self::Inhibit(module) => create!(module),
            #[cfg(feature = "keyboard")]
            Self::Keyboard(module) => create!(module),
            #[cfg(feature = "label")]
            Self::Label(module) => create!(module),
            #[cfg(feature = "launcher")]
            Self::Launcher(module) => create!(module),
            #[cfg(feature = "menu")]
            Self::Menu(module) => create!(module),
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
            #[cfg(feature = "tray")]
            Self::Tray(module) => create!(module),
            #[cfg(feature = "volume")]
            Self::Volume(module) => create!(module),
            #[cfg(feature = "workspaces")]
            Self::Workspaces(module) => create!(module),
        }
    }

    pub fn name(&self) -> String {
        match self {
            #[cfg(feature = "battery")]
            ModuleConfig::Battery(_) => "Battery",
            #[cfg(feature = "bindmode")]
            ModuleConfig::Bindmode(_) => "Bindmode",
            #[cfg(feature = "cairo")]
            ModuleConfig::Cairo(_) => "Cario",
            #[cfg(feature = "clipboard")]
            ModuleConfig::Clipboard(_) => "Clipboard",
            #[cfg(feature = "clock")]
            ModuleConfig::Clock(_) => "Clock",
            #[cfg(feature = "custom")]
            ModuleConfig::Custom(_) => "Custom",
            #[cfg(feature = "focused")]
            ModuleConfig::Focused(_) => "Focused",
            #[cfg(feature = "inhibit")]
            ModuleConfig::Inhibit(_) => "Inhibit",
            #[cfg(feature = "keyboard")]
            ModuleConfig::Keyboard(_) => "Keyboard",
            #[cfg(feature = "label")]
            ModuleConfig::Label(_) => "Label",
            #[cfg(feature = "launcher")]
            ModuleConfig::Launcher(_) => "Launcher",
            #[cfg(feature = "menu")]
            ModuleConfig::Menu(_) => "Menu",
            #[cfg(feature = "music")]
            ModuleConfig::Music(_) => "Music",
            #[cfg(feature = "network_manager")]
            ModuleConfig::NetworkManager(_) => "NetworkManager",
            #[cfg(feature = "notifications")]
            ModuleConfig::Notifications(_) => "Notifications",
            #[cfg(feature = "script")]
            ModuleConfig::Script(_) => "Script",
            #[cfg(feature = "sys_info")]
            ModuleConfig::SysInfo(_) => "SysInfo",
            #[cfg(feature = "tray")]
            ModuleConfig::Tray(_) => "Tray",
            #[cfg(feature = "volume")]
            ModuleConfig::Volume(_) => "Volume",
            #[cfg(feature = "workspaces")]
            ModuleConfig::Workspaces(_) => "Workspaces",
            // in case no modules are compiled
            #[allow(unreachable_patterns)]
            _ => "",
        }
        .to_string()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "extras", derive(JsonSchema))]
pub enum MonitorConfig {
    Single(BarConfig),
    Multiple(Vec<BarConfig>),
}

#[derive(Debug, Default, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(JsonSchema))]
pub enum BarPosition {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Default, Deserialize, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(JsonSchema))]
#[serde(default)]
pub struct MarginConfig {
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
    pub top: i32,
}

/// The following is a list of all top-level bar config options.
///
/// These options can either be written at the very top object of your config,
/// or within an object in the [monitors](#monitors) config,
/// depending on your [use-case](#2-pick-your-use-case).
///
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(JsonSchema))]
#[serde(default)]
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
    pub position: BarPosition,

    /// Whether to anchor the bar to the edges of the screen.
    /// Setting to false centers the bar.
    ///
    /// **Default**: `true`
    pub anchor_to_edges: bool,

    /// The bar's height in pixels.
    ///
    /// Note that GTK treats this as a target minimum,
    /// and if content inside the bar is over this,
    /// it will automatically expand to fit.
    ///
    /// **Default**: `42`
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
    #[serde(deserialize_with = "r#impl::deserialize_layer")]
    #[cfg_attr(feature = "extras", schemars(schema_with = "r#impl::schema_layer"))]
    pub layer: gtk_layer_shell::Layer,

    /// Whether the bar should reserve an exclusive zone around it.
    ///
    /// When true, this prevents windows from rendering in the same space
    /// as the bar, causing them to shift.
    ///
    /// **Default**: `true` unless `start_hidden` is set.
    pub exclusive_zone: Option<bool>,

    /// The size of the gap in pixels
    /// between the bar and the popup window.
    ///
    /// **Default**: `5`
    pub popup_gap: i32,

    /// Whether to enable autohide behaviour on the popup.
    ///
    /// When enabled, clicking outside the popup will close it.
    /// On some compositors, this may also aggressively steal mouse/keyboard focus.
    ///
    /// **Default**: `false`
    pub popup_autohide: bool,

    /// Whether the bar should be hidden when Ironbar starts.
    ///
    /// **Default**: `false`, unless `autohide` is set.
    pub start_hidden: Option<bool>,

    /// The duration in milliseconds before the bar is hidden after the cursor leaves.
    /// Leave unset to disable auto-hide behaviour.
    ///
    /// **Default**: `null`
    pub autohide: Option<u64>,

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
        Self {
            position: BarPosition::default(),
            margin: MarginConfig::default(),
            name: None,
            layer: gtk_layer_shell::Layer::Top,
            exclusive_zone: None,
            height: 42,
            start_hidden: None,
            autohide: None,
            start: None,
            center: None,
            end: None,
            anchor_to_edges: true,
            popup_gap: 5,
            popup_autohide: false,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "extras", derive(JsonSchema))]
#[serde(default)]
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
    /// Monitor names can be supplied in two formats:
    ///
    /// - Connector names (`DP-1`, `HDMI-2`)
    /// - Descriptions (`ASUSTek COMPUTER INC PA278QV M4LMQS060475`).
    ///   A `starts_with` is applied allowing you to omit part of the description if convenient.
    ///
    /// The config values can be either:
    ///
    /// - a single object, which denotes a single bar for that monitor,
    /// - an array of multiple objects, which denotes multiple for that monitor.
    ///
    /// Providing this option overrides the single, global `bar` option.
    pub monitors: Option<HashMap<String, MonitorConfig>>,

    /// The name of the GTK icon theme to use.
    /// Leave unset to use the default system theme.
    ///
    /// **Default**: `null`
    pub icon_theme: Option<String>,

    /// Map of app IDs (or classes) to icon names,
    /// overriding the app's default icon.
    ///
    /// **Default**: `{}`
    pub icon_overrides: HashMap<String, String>,

    /// The time in milliseconds to wait for a double-click.
    /// Can be set to a number (e.g., `250`) or `"gtk"` to use GTK's setting.
    ///
    /// **Default**: `250`
    #[serde(default)]
    pub double_click_time: DoubleClickTime,
}

/// Double-click time configuration
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DoubleClickTime {
    /// Use GTK's gtk-double-click-time setting
    Gtk,
    /// Milliseconds
    #[serde(untagged)]
    Ms(u64),
}

impl Default for DoubleClickTime {
    fn default() -> Self {
        Self::Ms(250)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl Default for ConfigLocation {
    fn default() -> Self {
        Self::Custom(Self::default_path())
    }
}

impl ConfigLocation {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_default()
            .to_path_buf()
            .join("ironbar/config")
    }

    #[cfg(not(feature = "cli"))]
    pub fn from_env(key: &str) -> Option<Self> {
        std::env::var(key).map(PathBuf::from).ok().map(Self::Custom)
    }
}

impl Config {
    #[cfg(feature = "config")]
    pub fn load(
        config_location: ConfigLocation,
        css_location: Option<ConfigLocation>,
    ) -> (Config, CssSource) {
        cfg_if! {
            if #[cfg(feature = "config+corn")] {
                const CONFIG_MINIMAL: (&str, FileFormat) = (include_str!("../../examples/minimal/config.corn"), FileFormat::Corn);
                const CONFIG_DESKTOP: (&str, FileFormat) = (include_str!("../../examples/desktop/config.corn"), FileFormat::Corn);
            } else if #[cfg(feature = "config+json")] {
                const CONFIG_MINIMAL: (&str, FileFormat) = (include_str!("../../examples/minimal/config.json"), FileFormat::Json);
                const CONFIG_DESKTOP: (&str, FileFormat) = (include_str!("../../examples/desktop/config.json"), FileFormat::Json);
            } else if #[cfg(feature = "config+yaml")] {
                const CONFIG_MINIMAL: (&str, FileFormat) = (include_str!("../../examples/minimal/config.yaml"), FileFormat::Yaml);
                const CONFIG_DESKTOP: (&str, FileFormat) = (include_str!("../../examples/desktop/config.yaml"), FileFormat::Yaml);
            } else if #[cfg(feature = "config+toml")] {
                const CONFIG_MINIMAL: (&str, FileFormat) = (include_str!("../../examples/minimal/config.toml"), FileFormat::Toml);
                const CONFIG_DESKTOP: (&str, FileFormat) = (include_str!("../../examples/desktop/config.toml"), FileFormat::Toml);
            }
        }

        const CSS_MINIMAL: CssSource =
            CssSource::String(include_str!("../../examples/minimal/style.css"));

        const CSS_DESKTOP: CssSource =
            CssSource::String(include_str!("../../examples/desktop/style.css"));

        let config_builder = config::Config::builder();

        let css_source = match css_location.unwrap_or_else(|| config_location.clone()) {
            ConfigLocation::Minimal => CSS_MINIMAL,
            ConfigLocation::Desktop => CSS_DESKTOP,
            ConfigLocation::Custom(mut path) => {
                if path.is_dir() {
                    path = path.join("style.css")
                } else if path.extension().is_none_or(|ext| ext != "css") {
                    path = path.parent().unwrap_or(&path).join("style.css");
                };

                if path.exists() {
                    CssSource::File(path)
                } else {
                    error!(
                        "styles at '{}' not found, falling back to minimal theme",
                        path.display()
                    );
                    CSS_MINIMAL
                }
            }
        };

        let config_builder = match config_location {
            ConfigLocation::Minimal => config_builder
                .add_source(config::File::from_str(CONFIG_MINIMAL.0, CONFIG_MINIMAL.1)),
            ConfigLocation::Desktop => config_builder
                .add_source(config::File::from_str(CONFIG_DESKTOP.0, CONFIG_DESKTOP.1)),
            ConfigLocation::Custom(path) => config_builder.add_source(config::File::from(path)),
        };

        let mut config: Config = config_builder
            .add_source(config::Environment::with_prefix("IRONBAR_"))
            .build()
            .and_then(|conf| conf.try_deserialize())
            .unwrap_or_else(|err| {
                error!("Error loading config: {err:?}");
                config::Config::builder()
                    .add_source(config::File::from_str(CONFIG_MINIMAL.0, CONFIG_MINIMAL.1))
                    .build()
                    .expect("should be a valid config")
                    .try_deserialize()
                    .expect("should be a valid config")
            });

        #[cfg(feature = "ipc")]
        if let Some(ironvars) = config.ironvar_defaults.take() {
            use crate::ironvar::WritableNamespace;

            let variable_manager = Ironbar::variable_manager();
            for (k, v) in ironvars {
                if variable_manager.set(&k, v).is_err() {
                    warn!("Ignoring invalid ironvar: '{k}'");
                }
            }
        }

        // Store the double-click time globally
        // GTK's setting will be set lazily on first use (after GTK is initialized)
        set_double_click_time(config.double_click_time.clone());

        (config, css_source)
    }

    #[cfg(not(feature = "config"))]
    pub fn load(
        config_location: ConfigLocation,
        css_location: Option<ConfigLocation>,
    ) -> (Config, CssSource) {
        panic!(
            "Ironbar has been configured without config support. This won't work. Please reconfigure with at least one `config` feature flag enabled."
        )
    }
}
