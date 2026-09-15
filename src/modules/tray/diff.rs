//! Incremental application of dbusmenu property updates (`MenuDiff`) to a
//! retained menu model.
//!
//! The `system-tray` crate delivers post-interaction changes — a radio
//! selection moving to another entry, a submenu label changing — as
//! [`MenuDiff`]s keyed by the *global* dbusmenu item id, which can sit at any
//! depth in the menu tree. The crate's own `apply_menu_diffs` only walks the
//! top-level items, so nested items (e.g. a radio group inside a submenu) never
//! receive their updates. We therefore walk the whole tree and patch the item
//! whose id matches, recursing into every submenu.

use system_tray::menu::{MenuDiff, MenuItem, MenuItemUpdate};

/// Applies every diff to the matching item anywhere in `items`, recursing into
/// nested submenus. A diff whose id matches no item is ignored.
///
/// The `remove` field of a diff (properties reset to their default) is not
/// applied — this matches the property set the `system-tray` crate itself
/// understands, and the reported failures are all `update`s (toggle-state,
/// label).
pub(crate) fn apply_menu_diffs(items: &mut [MenuItem], diffs: &[MenuDiff]) {
    for item in items {
        if let Some(diff) = diffs.iter().find(|d| d.id == item.id) {
            apply_menu_item_update(item, &diff.update);
        }
        apply_menu_diffs(&mut item.submenu, diffs);
    }
}

/// Overwrites exactly the fields the update carries, leaving the rest intact.
fn apply_menu_item_update(item: &mut MenuItem, update: &MenuItemUpdate) {
    if let Some(label) = &update.label {
        item.label.clone_from(label);
    }
    if let Some(enabled) = update.enabled {
        item.enabled = enabled;
    }
    if let Some(visible) = update.visible {
        item.visible = visible;
    }
    if let Some(icon_name) = &update.icon_name {
        item.icon_name.clone_from(icon_name);
    }
    if let Some(icon_data) = &update.icon_data {
        item.icon_data.clone_from(icon_data);
    }
    if let Some(toggle_state) = update.toggle_state {
        item.toggle_state = toggle_state;
    }
    if let Some(disposition) = update.disposition {
        item.disposition = disposition;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use system_tray::menu::{MenuDiff, MenuItem, MenuItemUpdate, ToggleState, ToggleType};

    fn radio(id: i32, label: &str, state: ToggleState) -> MenuItem {
        MenuItem {
            id,
            label: Some(label.to_string()),
            enabled: true,
            visible: true,
            toggle_type: ToggleType::Radio,
            toggle_state: state,
            ..Default::default()
        }
    }

    fn toggle_state_diff(id: i32, state: ToggleState) -> MenuDiff {
        MenuDiff {
            id,
            update: MenuItemUpdate {
                toggle_state: Some(state),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// The reported bug: a radio group nested inside a submenu. Clicking an
    /// entry moves the `toggle-state` from one item to another; dbusmenu
    /// delivers this as `MenuDiff`s keyed by the (nested) item ids, so the menu
    /// model must reflect the moved selection.
    #[test]
    fn applies_toggle_state_move_to_nested_radio_group() {
        // given: a submenu containing a radio group, "A" selected, "B" not
        let mut items = vec![MenuItem {
            id: 1,
            label: Some("parent".to_string()),
            enabled: true,
            visible: true,
            children_display: Some("submenu".to_string()),
            submenu: vec![
                radio(10, "A", ToggleState::On),
                radio(11, "B", ToggleState::Off),
            ],
            ..Default::default()
        }];

        // when: the user clicks "B" — A turns off, B turns on
        let diffs = vec![
            toggle_state_diff(10, ToggleState::Off),
            toggle_state_diff(11, ToggleState::On),
        ];
        apply_menu_diffs(&mut items, &diffs);

        // then: the nested radio group reflects the new selection
        let submenu = &items[0].submenu;
        assert_eq!(
            submenu[0].toggle_state,
            ToggleState::Off,
            "clicked-away entry A should be deselected"
        );
        assert_eq!(
            submenu[1].toggle_state,
            ToggleState::On,
            "clicked entry B should be selected"
        );
    }

    /// A label change (e.g. a submenu summary reflecting the new selection)
    /// must be applied.
    #[test]
    fn applies_label_update() {
        let mut items = vec![radio(5, "old", ToggleState::Off)];
        let diffs = vec![MenuDiff {
            id: 5,
            update: MenuItemUpdate {
                label: Some(Some("new".to_string())),
                ..Default::default()
            },
            ..Default::default()
        }];

        apply_menu_diffs(&mut items, &diffs);

        assert_eq!(items[0].label.as_deref(), Some("new"));
    }

    /// A diff whose id matches no item is a harmless no-op.
    #[test]
    fn ignores_unknown_id() {
        let mut items = vec![radio(1, "A", ToggleState::On)];
        let diffs = vec![toggle_state_diff(999, ToggleState::Off)];

        apply_menu_diffs(&mut items, &diffs);

        assert_eq!(items[0].toggle_state, ToggleState::On);
    }
}
