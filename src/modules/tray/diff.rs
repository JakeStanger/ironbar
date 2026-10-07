//! Incremental application of dbusmenu property updates (`MenuDiff`) to a
//! retained menu model.

use system_tray::menu::{Disposition, MenuDiff, MenuItem, ToggleState};

/// Applies every diff to the matching item anywhere in `items`, recursing into
/// nested submenus.
pub(crate) fn apply_menu_diffs(items: &mut [MenuItem], diffs: &[MenuDiff]) {
    for item in items {
        if let Some(diff) = diffs.iter().find(|d| d.id == item.id) {
            apply_menu_item_update(item, diff);
        }

        apply_menu_diffs(&mut item.submenu, diffs);
    }
}

/// Applies a single diff to a menu item,
/// mutating it to update its values,
/// and unsetting any removed fields.
fn apply_menu_item_update(item: &mut MenuItem, diff: &MenuDiff) {
    let update = &diff.update;
    let remove = &diff.remove;

    if let Some(label) = &update.label {
        item.label.clone_from(label);
    }
    if remove.contains(&String::from("label")) {
        item.label = None;
    }

    if let Some(enabled) = update.enabled {
        item.enabled = enabled;
    }
    if remove.contains(&String::from("enabled")) {
        item.enabled = true;
    }

    if let Some(visible) = update.visible {
        item.visible = visible;
    }
    if remove.contains(&String::from("visible")) {
        item.visible = true;
    }

    if let Some(icon_name) = &update.icon_name {
        item.icon_name.clone_from(icon_name);
    }
    if remove.contains(&String::from("icon-name")) {
        item.icon_name = None;
    }

    if let Some(icon_data) = &update.icon_data {
        item.icon_data.clone_from(icon_data);
    }
    if remove.contains(&String::from("icon-data")) {
        item.icon_data = None;
    }

    if let Some(toggle_state) = update.toggle_state {
        item.toggle_state = toggle_state;
    }
    if remove.contains(&String::from("toggle-state")) {
        item.toggle_state = ToggleState::Indeterminate;
    }

    if let Some(disposition) = update.disposition {
        item.disposition = disposition;
    }
    if remove.contains(&String::from("disposition")) {
        item.disposition = Disposition::Normal;
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

    /// Tests against a radio group nested inside a submenu.
    ///
    /// Clicking an entry moves the `toggle-state` from one item to another.
    /// `dbusmenu` delivers this as `MenuDiff`s keyed by the (nested) item ids.
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
