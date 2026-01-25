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

        if old.icon_theme != new.icon_theme {
            diff.icon_theme = Diff::Updated(());
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
        let start_len_eq = old.start.as_ref().map(Vec::len).unwrap_or_default()
            == new.start.as_ref().map(Vec::len).unwrap_or_default();

        let center_len_eq = old.center.as_ref().map(Vec::len).unwrap_or_default()
            == new.center.as_ref().map(Vec::len).unwrap_or_default();

        let end_len_eq = old.end.as_ref().map(Vec::len).unwrap_or_default()
            == new.end.as_ref().map(Vec::len).unwrap_or_default();

        // TODO: Don't like that this needs manually maintaining
        let recreate_required = old.autohide != new.autohide
            || old.popup_autohide != new.popup_autohide
            || old.anchor_to_edges != new.anchor_to_edges
            || old.exclusive_zone != new.exclusive_zone
            || old.start_hidden != new.start_hidden
            || old.position != new.position
            || old.height != new.height
            || old.layer != new.layer
            || old.name != new.name
            || old.popup_gap != new.popup_gap
            || old.margin != new.margin
            || !start_len_eq
            || !center_len_eq
            || !end_len_eq;

        if recreate_required {
            BarDiff::Recreate
        } else {
            let mut diff = BarDiffDetails::default();

            if let (Some(modules_old), Some(modules_new)) = (old.start.as_ref(), new.start.as_ref())
            {
                diff.start = modules_old
                    .iter()
                    .zip(modules_new)
                    .enumerate()
                    .filter(|(_, (old, new))| old != new)
                    .map(|(i, _)| i)
                    .collect();
            }

            if let (Some(modules_old), Some(modules_new)) =
                (old.center.as_ref(), new.center.as_ref())
            {
                diff.center = modules_old
                    .iter()
                    .zip(modules_new)
                    .enumerate()
                    .filter(|(_, (old, new))| old != new)
                    .map(|(i, _)| i)
                    .collect();
            }

            if let (Some(modules_old), Some(modules_new)) = (old.end.as_ref(), new.end.as_ref()) {
                diff.end = modules_old
                    .iter()
                    .zip(modules_new)
                    .enumerate()
                    .filter(|(_, (old, new))| old != new)
                    .map(|(i, _)| i)
                    .collect();
            }

            BarDiff::Reload(diff)
        }
    }
}
