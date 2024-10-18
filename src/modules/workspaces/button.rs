use super::open_state::OpenState;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::new_icon_button;
use crate::modules::workspaces::WorkspaceItemContext;
use crate::try_send;
use gtk::prelude::*;
use gtk::Button as GtkButton;

#[derive(Debug, Clone)]
pub struct Button {
    button: GtkButton,
    workspace_id: i64,
    open_state: OpenState,
}

impl Button {
    pub fn new(id: i64, name: &str, open_state: OpenState, context: &WorkspaceItemContext) -> Self {
        let label = context.name_map.get(name).map_or(name, String::as_str);

        let button = new_icon_button(label, &context.icon_theme, context.icon_size);
        button.set_widget_name(name);
        button.add_class("item");

        let tx = context.tx.clone();

        let name = name.to_string();
        button.connect_clicked(move |_item| {
            try_send!(tx, name.clone());
        });

        let btn = Self {
            button,
            workspace_id: id,
            open_state,
        };

        btn.update_classes();
        btn
    }

    pub fn button(&self) -> &GtkButton {
        &self.button
    }

    pub fn set_open_state(&mut self, open_state: OpenState) {
        self.open_state = open_state;
        self.update_classes();
    }

    pub fn workspace_id(&self) -> i64 {
        self.workspace_id
    }

    pub fn set_workspace_id(&mut self, id: i64) {
        self.workspace_id = id;
    }

    /// Adds/removes styling classes according to the open state.
    pub fn update_classes(&self) {
        if self.open_state.is_visible() {
            self.button.add_class("visible");
        } else {
            self.button.remove_class("visible");
        }

        if self.open_state == OpenState::Focused {
            self.button.add_class("focused");
        } else {
            self.button.remove_class("focused");
        }

        if self.open_state == OpenState::Closed {
            self.button.add_class("inactive");
        } else {
            self.button.remove_class("inactive");
        }
    }
}
