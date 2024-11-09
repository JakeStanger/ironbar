use system_tray::menu::{MenuItem, ToggleState};

/// Diff change type and associated info.
#[derive(Debug, Clone)]
pub enum Diff {
    Add(MenuItem),
    Update(i32, MenuItemDiff),
    Remove(i32),
}

/// Diff info to be applied to an existing menu item as an update.
#[derive(Debug, Clone)]
pub struct MenuItemDiff {
    /// Text of the item,
    pub label: Option<Option<String>>,
    /// Whether the item can be activated or not.
    pub enabled: Option<bool>,
    /// True if the item is visible in the menu.
    pub visible: Option<bool>,
    /// Icon name of the item, following the freedesktop.org icon spec.
    pub icon_name: Option<Option<String>>,
    /// PNG icon data.
    pub icon_data: Option<Option<Vec<u8>>>,
    /// Describe the current state of a "togglable" item. Can be one of:
    ///   - Some(true): on
    ///   - Some(false): off
    ///   - None: indeterminate
    pub toggle_state: Option<ToggleState>,
    /// A submenu for this item, typically this would ve revealed to the user by hovering the current item
    pub submenu: Vec<Diff>,
}

impl MenuItemDiff {
    fn new(old: &MenuItem, new: &MenuItem) -> Self {
        macro_rules! diff {
            ($field:ident) => {
                if old.$field == new.$field {
                    None
                } else {
                    Some(new.$field)
                }
            };

            (&$field:ident) => {
                if &old.$field == &new.$field {
                    None
                } else {
                    Some(new.$field.clone())
                }
            };
        }

        Self {
            label: diff!(&label),
            enabled: diff!(enabled),
            visible: diff!(visible),
            icon_name: diff!(&icon_name),
            icon_data: diff!(&icon_data),
            toggle_state: diff!(toggle_state),
            submenu: get_diffs(&old.submenu, &new.submenu),
        }
    }

    /// Whether this diff contains any changes
    fn has_diff(&self) -> bool {
        self.label.is_some()
            || self.enabled.is_some()
            || self.visible.is_some()
            // || self.icon_name.is_some()
            || self.toggle_state.is_some()
            || !self.submenu.is_empty()
    }
}

/// Gets a diff set between old and new state.
pub fn get_diffs(old: &[MenuItem], new: &[MenuItem]) -> Vec<Diff> {
    let mut diffs = vec![];

    for new_item in new {
        let old_item = old.iter().find(|&item| item.id == new_item.id);

        let diff = match old_item {
            Some(old_item) => {
                let item_diff = MenuItemDiff::new(old_item, new_item);
                if item_diff.has_diff() {
                    Some(Diff::Update(old_item.id, item_diff))
                } else {
                    None
                }
            }
            None => Some(Diff::Add(new_item.clone())),
        };

        if let Some(diff) = diff {
            diffs.push(diff);
        }
    }

    for old_item in old {
        let new_item = new.iter().find(|&item| item.id == old_item.id);
        if new_item.is_none() {
            diffs.push(Diff::Remove(old_item.id));
        }
    }

    diffs
}
