use super::open_state::OpenState;
use crate::channels::AsyncSenderExt;
use crate::image::IconButton;
use crate::modules::workspaces::{WorkspaceClickEvent, WorkspaceItemContext};
use glib::signal::SignalHandlerId;
use gtk::Button as GtkButton;
use gtk::gdk;
use gtk::prelude::*;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Button {
    button: IconButton,
    workspace_id: i64,
    left_conn_id: Option<SignalHandlerId>,
    right_click: gtk::GestureClick,
    right_conn_id: Option<SignalHandlerId>,
    tx: mpsc::Sender<WorkspaceClickEvent>,
}

impl Button {
    pub fn new(
        id: i64,
        index: i64,
        name: &str,
        open_state: OpenState,
        context: &WorkspaceItemContext,
    ) -> Self {
        let label = context.format_label(name, index);

        let button = IconButton::new(&label, context.icon_size, context.image_provider.clone());
        button.set_widget_name(name);
        button.add_css_class("item");

        let tx = context.tx.clone();

        let left_conn_id = button.connect_clicked(move |_item| {
            tx.send_spawn(WorkspaceClickEvent::Left(id));
        });

        let right_click = gtk::GestureClick::new();
        right_click.set_button(gdk::BUTTON_SECONDARY);

        let tx = context.tx.clone();
        let right_conn_id = right_click.connect_pressed(move |_gesture, _, _, _| {
            tx.send_spawn(WorkspaceClickEvent::Right(id));
        });

        button.add_controller(right_click.clone());

        let btn = Self {
            button,
            workspace_id: id,
            left_conn_id: Some(left_conn_id),
            right_click,
            right_conn_id: Some(right_conn_id),
            tx: context.tx.clone(),
        };

        btn.set_open_state(open_state);
        btn
    }

    pub fn button(&self) -> &GtkButton {
        &self.button
    }

    pub fn set_label(&self, label: &str) {
        self.button.set_label(label);
    }

    pub fn set_open_state(&self, open_state: OpenState) {
        if open_state.is_visible() {
            self.button.add_css_class("visible");
        } else {
            self.button.remove_css_class("visible");
        }

        if open_state == OpenState::Focused {
            self.button.add_css_class("focused");
        } else {
            self.button.remove_css_class("focused");
        }

        if open_state == OpenState::Closed {
            self.button.add_css_class("inactive");
        } else {
            self.button.remove_css_class("inactive");
        }
    }

    pub fn set_urgent(&self, urgent: bool) {
        if urgent {
            self.button.add_css_class("urgent");
        } else {
            self.button.remove_css_class("urgent");
        }
    }

    pub fn workspace_id(&self) -> i64 {
        self.workspace_id
    }

    pub fn set_workspace_id(&mut self, id: i64) {
        self.workspace_id = id;
        if let Some(conn_id) = self.left_conn_id.take() {
            self.button.disconnect(conn_id);
        }
        if let Some(conn_id) = self.right_conn_id.take() {
            self.right_click.disconnect(conn_id);
        }
        let tx = self.tx.clone();
        let left_conn_id = self.button.connect_clicked(move |_item| {
            tx.send_spawn(WorkspaceClickEvent::Left(id));
        });
        self.left_conn_id = Some(left_conn_id);

        let tx = self.tx.clone();
        let right_conn_id = self.right_click.connect_pressed(move |_gesture, _, _, _| {
            tx.send_spawn(WorkspaceClickEvent::Right(id));
        });
        self.right_conn_id = Some(right_conn_id);
    }
}
