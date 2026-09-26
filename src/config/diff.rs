use super::{BarConfig, Config, MonitorConfig};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Diff<T> {
    Added,
    Removed,
    Updated(T),
    #[default]
    NoChange,
}

#[derive(Debug, Clone, Default)]
pub struct ConfigDiff {
    pub icon_theme: Diff<()>,
    pub icon_overrides: Diff<()>,
    pub bar_diff: Diff<BarDiff>,
    pub monitor_diff: Diff<MonitorDiffDetails>,
}

#[derive(Debug, Clone)]
pub enum MonitorDiff {
    Recreate,
    UpdateSingle(BarDiff),
    UpdateMultiple(Vec<BarDiff>),
}

#[derive(Debug, Clone, Default)]
pub struct MonitorDiffDetails {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: HashMap<String, MonitorDiff>,
}

impl MonitorDiffDetails {
    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BarDiff {
    Recreate,
    Reload(BarDiffDetails),
}

impl BarDiff {
    fn is_empty(&self) -> bool {
        match self {
            BarDiff::Recreate => false,
            BarDiff::Reload(diff) => diff.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BarDiffDetails {
    pub height: bool,
    pub position: bool,
    pub anchor_to_edges: bool,
    pub margin: bool,
    pub layer: bool,
    pub popup_gap: bool,

    pub start: Vec<usize>,
    pub center: Vec<usize>,
    pub end: Vec<usize>,
}

impl BarDiffDetails {
    fn is_empty(&self) -> bool {
        self.start.is_empty() && self.center.is_empty() && self.end.is_empty()
    }
}

impl ConfigDiff {
    pub fn diff(old: &Config, new: &Config) -> ConfigDiff {
        let mut diff = ConfigDiff::default();

        // Note - `ironvar_defaults` and `double_click_time` are automatically reloaded
        // when the config is loaded, so are not included here
        // TODO: Move the behaviours out of the config loader

        if old.icon_theme != new.icon_theme {
            diff.icon_theme = Diff::Updated(());
        }

        if old.icon_overrides != new.icon_overrides {
            diff.icon_overrides = Diff::Updated(());
        }

        diff.bar_diff = {
            let diff = BarDiff::diff(&old.bar, &new.bar);
            if diff.is_empty() {
                Diff::NoChange
            } else {
                Diff::Updated(diff)
            }
        };

        diff.monitor_diff = match (&old.monitors, &new.monitors) {
            (Some(old_monitors), Some(new_monitors)) => {
                if old_monitors == new_monitors {
                    Diff::NoChange
                } else {
                    let mut diff = MonitorDiffDetails::default();

                    for (old_key, old) in old_monitors {
                        if let Some(new) = new_monitors.get(old_key) {
                            let monitor_diff = MonitorDiff::diff(old, new);
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
                        Diff::NoChange
                    } else {
                        Diff::Updated(diff)
                    }
                }
            }
            (Some(_), None) => Diff::Removed,
            (None, Some(_)) => Diff::Added,
            (None, None) => Diff::NoChange,
        };

        diff
    }
}

impl MonitorDiff {
    fn diff(old: &MonitorConfig, new: &MonitorConfig) -> Self {
        match (old, new) {
            (MonitorConfig::Single(old), MonitorConfig::Single(new)) => {
                Self::UpdateSingle(BarDiff::diff(old, new))
            }
            (MonitorConfig::Multiple(old_bars), MonitorConfig::Multiple(new_bars)) => {
                if old_bars.len() == new_bars.len() {
                    let diffs = old_bars
                        .iter()
                        .zip(new_bars)
                        .map(|(old, new)| BarDiff::diff(old, new))
                        .collect();

                    Self::UpdateMultiple(diffs)
                } else {
                    Self::Recreate
                }
            }
            _ => Self::Recreate,
        }
    }
}

impl BarDiff {
    pub fn diff(old: &BarConfig, new: &BarConfig) -> Self {
        old.hot_reload_diff(new)
    }
}
