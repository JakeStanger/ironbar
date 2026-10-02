pub(crate) use super::{BarConfig, BarConfigDiff, Config, MonitorConfig};
use std::collections::HashMap;

/// Top-level config diff.
///
/// Note - `ironvar_defaults` and `double_click_time` are automatically reloaded
/// when the config is loaded, so are not included here.
#[derive(Debug, Clone)]
pub struct ConfigDiff {
    pub icon_theme: bool,
    pub icon_overrides: bool,
    /// Bar diff if individual monitors are not configured.
    /// Empty if the `monitors` key is set inside the config.
    pub bar_diff: BarDiffAction,
    /// Diff for all monitors, with bar diffs mapped inside,
    /// if individual monitors are configured.
    pub monitor_diff: MonitorDiff,
}

/// Diff for the `monitors` key.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum MonitorDiff {
    /// The monitors key has been set. A full reload of all bars is required.
    Added,
    /// The monitors key has been removed. A full reload of all bars is required.
    Removed,
    /// The existing monitors map has been updated. Diff attached.
    Updated(MonitorUpdateDiff),
    /// Monitors map has not changed.
    #[default]
    NoChange,
}

/// Action to take for an individual monitor update.
#[derive(Debug, Clone, PartialEq)]
pub enum MonitorUpdateDiffAction {
    /// All bars on this monitor should be recreated.
    Reload,
    /// A single bar on this monitor should be updated,
    /// with the diff attached.
    UpdateSingle(BarDiffAction),
    /// Several/all bars on this monitor should be updated,
    /// with the diffs attached.
    ///
    /// Diffs may not be in any specific order,
    /// so bars must be looked up per diff.
    UpdateMultiple(Vec<BarDiffAction>),
}

/// Details for an individual monitor change.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MonitorUpdateDiff {
    /// A list of monitor names that have been added.
    pub added: Vec<String>,
    /// A list of monitor names that have been removed.
    pub removed: Vec<String>,
    /// Details for each individual monitor update
    /// for all pre-existing monitors.
    pub updated: HashMap<String, MonitorUpdateDiffAction>,
}

impl MonitorUpdateDiff {
    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }
}

/// Action to take for an individual bar update.
#[derive(Debug, Clone, PartialEq)]
pub enum BarDiffAction {
    /// The bar should be removed and fully recreated.
    Reload,
    /// The bar should be updated with the attached diff.
    Update(BarConfigDiff),
}

impl BarDiffAction {
    pub fn is_empty(&self) -> bool {
        match self {
            BarDiffAction::Reload => false,
            BarDiffAction::Update(diff) => diff.is_empty(),
        }
    }
}

impl ConfigDiff {
    pub fn diff(old: &Config, new: &Config) -> ConfigDiff {
        let monitor_diff = match (&old.monitors, &new.monitors) {
            (Some(old_monitors), Some(new_monitors)) => {
                if old_monitors == new_monitors {
                    MonitorDiff::NoChange
                } else {
                    let mut diff = MonitorUpdateDiff::default();

                    for (old_key, old) in old_monitors {
                        if let Some(new) = new_monitors.get(old_key) {
                            let monitor_diff = MonitorUpdateDiffAction::diff(old, new);
                            diff.updated.insert(old_key.clone(), monitor_diff);
                        } else {
                            diff.removed.push(old_key.clone());
                        }
                    }

                    for new_key in new_monitors.keys() {
                        if !old_monitors.contains_key(new_key) {
                            diff.added.push(new_key.clone());
                        }
                    }

                    if diff.is_empty() {
                        MonitorDiff::NoChange
                    } else {
                        MonitorDiff::Updated(diff)
                    }
                }
            }
            (Some(_), None) => MonitorDiff::Removed,
            (None, Some(_)) => MonitorDiff::Added,
            (None, None) => MonitorDiff::NoChange,
        };

        ConfigDiff {
            icon_theme: old.icon_theme != new.icon_theme,
            icon_overrides: old.icon_overrides != new.icon_overrides,
            bar_diff: BarDiffAction::diff(&old.bar, &new.bar),
            monitor_diff,
        }
    }
}

impl MonitorUpdateDiffAction {
    fn diff(old: &MonitorConfig, new: &MonitorConfig) -> Self {
        match (old, new) {
            (MonitorConfig::Single(old), MonitorConfig::Single(new)) => {
                Self::UpdateSingle(BarDiffAction::diff(old, new))
            }
            (MonitorConfig::Multiple(old_bars), MonitorConfig::Multiple(new_bars)) => {
                if old_bars.len() == new_bars.len() {
                    let diffs = old_bars
                        .iter()
                        .zip(new_bars)
                        .map(|(old, new)| BarDiffAction::diff(old, new))
                        .collect();

                    Self::UpdateMultiple(diffs)
                } else {
                    Self::Reload
                }
            }
            _ => Self::Reload,
        }
    }
}

impl BarDiffAction {
    pub fn diff(old: &BarConfig, new: &BarConfig) -> Self {
        old.hot_reload_diff(new)
    }
}
