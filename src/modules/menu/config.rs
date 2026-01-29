use crate::config::default;
use crate::config::{CommonConfig, TruncateMode};
use crate::modules::menu::{MenuEntry, XdgSection};
use indexmap::IndexMap;
use serde::Deserialize;

/// An individual entry in the main menu section.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MenuConfig {
    /// Contains all applications matching the configured `categories`.
    XdgEntry(XdgEntry),
    /// Contains all applications not covered by `xdg_entry` categories.
    XdgOther,
    /// Individual shell command entry.
    Custom(CustomEntry),
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct XdgEntry {
    /// Text to display on the button.
    #[serde(default)]
    pub label: String,

    /// Name of the image icon to show next to the label.
    #[serde(default)]
    pub icon: Option<String>,

    /// XDG categories the associated submenu should contain.
    #[serde(default)]
    pub categories: Vec<String>,
}

/// Individual shell command entry.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct CustomEntry {
    /// Text to display on the button.
    #[serde(default)]
    pub label: String,

    /// Name of the image icon to show next to the label.
    ///
    /// **Default**: `null`
    pub icon: Option<String>,

    /// Shell command to execute when the button is clicked.
    /// This is run using `sh -c`.
    #[serde(default)]
    pub on_click: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct MenuModule {
    /// Items to add to the start of the main menu.
    ///
    /// **Default**: `[]`
    pub(super) start: Vec<MenuConfig>,

    /// Items to add to the start of the main menu.
    ///
    /// By default, this shows a number of XDG entries
    /// that should cover all common applications.
    ///
    /// **Default**: See `examples/menu/default`
    pub(super) center: Vec<MenuConfig>,

    /// Items to add to the end of the main menu.
    ///
    /// **Default**: `[]`
    pub(super) end: Vec<MenuConfig>,

    /// Fixed height of the menu.
    ///
    /// When set, if the number of (sub)menu entries exceeds this value,
    /// a scrollbar will be shown.
    ///
    /// Leave null to resize dynamically.
    ///
    /// **Default**: `null`
    pub(super) height: Option<i32>,

    /// Fixed width of the menu.
    ///
    /// Can be used with `truncate` options
    /// to customise how item labels are truncated.
    ///
    /// **Default**: `null`
    pub(super) width: Option<i32>,

    /// Label to show on the menu button on the bar.
    ///
    /// **Default**: `≡`
    pub(super) label: Option<String>,

    /// Icon to show on the menu button on the bar.
    ///
    /// **Default**: `null`
    pub(super) label_icon: Option<String>,

    /// Size of the `label_icon` image.
    pub(super) label_icon_size: i32,

    /// Size of the `app_icon_size` images.
    pub(super) app_icon_size: i32,

    // -- common --
    /// Truncate options to apply to (sub)menu item labels.
    ///
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `Auto (end)`
    pub(super) truncate: TruncateMode,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,

    /// Command used to launch applications.
    ///
    /// **Default**: `gtk-launch`
    pub launch_command: String,
}

impl Default for MenuModule {
    fn default() -> Self {
        MenuModule {
            start: vec![],
            center: default_menu(),
            end: vec![],
            height: None,
            width: None,
            truncate: TruncateMode::default(),
            label: Some("≡".to_string()),
            label_icon: None,
            label_icon_size: default::IconSize::Tiny as i32,
            app_icon_size: default::IconSize::Tiny as i32,
            common: Some(CommonConfig::default()),
            launch_command: default::launch_command(),
        }
    }
}

fn default_menu() -> Vec<MenuConfig> {
    vec![
        MenuConfig::XdgEntry(XdgEntry {
            label: "Accessories".to_string(),
            icon: Some("accessories".to_string()),
            categories: vec![
                "Accessibility".to_string(),
                "Core".to_string(),
                "Legacy".to_string(),
                "Utility".to_string(),
            ],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Development".to_string(),
            icon: Some("applications-development".to_string()),
            categories: vec!["Development".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Education".to_string(),
            icon: Some("applications-education".to_string()),
            categories: vec!["Education".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Games".to_string(),
            icon: Some("applications-games".to_string()),
            categories: vec!["Game".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Graphics".to_string(),
            icon: Some("applications-graphics".to_string()),
            categories: vec!["Graphics".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Multimedia".to_string(),
            icon: Some("applications-multimedia".to_string()),
            categories: vec![
                "Audio".to_string(),
                "Video".to_string(),
                "AudioVideo".to_string(),
            ],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Network".to_string(),
            icon: Some("applications-internet".to_string()),
            categories: vec!["Network".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Office".to_string(),
            icon: Some("applications-office".to_string()),
            categories: vec!["Office".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Science".to_string(),
            icon: Some("applications-science".to_string()),
            categories: vec!["Science".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "System".to_string(),
            icon: Some("applications-system".to_string()),
            categories: vec!["Emulator".to_string(), "System".to_string()],
        }),
        MenuConfig::XdgOther,
        MenuConfig::XdgEntry(XdgEntry {
            label: "Settings".to_string(),
            icon: Some("preferences-system".to_string()),
            categories: vec!["Settings".to_string(), "Screensaver".to_string()],
        }),
    ]
}

pub const OTHER_LABEL: &str = "Other";

pub fn parse_config(
    section_config: Vec<MenuConfig>,
    sections_by_cat: &mut IndexMap<String, Vec<String>>,
) -> IndexMap<String, MenuEntry> {
    section_config
        .into_iter()
        .map(|entry_config| match entry_config {
            MenuConfig::XdgEntry(entry) => {
                entry.categories.into_iter().for_each(|cat| {
                    let existing = sections_by_cat.get_mut(&cat);

                    if let Some(existing) = existing {
                        existing.push(entry.label.clone());
                    } else {
                        sections_by_cat.insert(cat, vec![entry.label.clone()]);
                    }
                });

                (
                    entry.label.clone(),
                    MenuEntry::Xdg(XdgSection {
                        label: entry.label,
                        icon: entry.icon,
                        applications: IndexMap::new(),
                    }),
                )
            }
            MenuConfig::XdgOther => (
                OTHER_LABEL.to_string(),
                MenuEntry::Xdg(XdgSection {
                    label: OTHER_LABEL.to_string(),
                    icon: Some("applications-other".to_string()),
                    applications: IndexMap::new(),
                }),
            ),
            MenuConfig::Custom(entry) => (
                entry.label.clone(),
                MenuEntry::Custom(CustomEntry {
                    icon: entry.icon,
                    label: entry.label,
                    on_click: entry.on_click,
                }),
            ),
        })
        .collect()
}
