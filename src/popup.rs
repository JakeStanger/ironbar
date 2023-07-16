use std::collections::HashMap;

use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Orientation};
use tracing::debug;

use crate::config::BarPosition;
use crate::gtk_helpers::{IronbarGtkExt, WidgetGeometry};
use crate::modules::{ModuleInfo, ModulePopupParts, PopupButton};
use crate::unique_id::get_unique_usize;

#[derive(Debug, Clone)]
pub struct Popup {
    pub window: ApplicationWindow,
    pub cache: HashMap<usize, (String, ModulePopupParts)>,
    monitor: Monitor,
    pos: BarPosition,
    current_widget: Option<usize>,
}

impl Popup {
    /// Creates a new popup window.
    /// This includes setting up gtk-layer-shell
    /// and an empty `gtk::Box` container.
    pub fn new(module_info: &ModuleInfo, gap: i32) -> Self {
        let pos = module_info.bar_position;
        let orientation = pos.get_orientation();

        let win = ApplicationWindow::builder()
            .application(module_info.app)
            .build();

        gtk_layer_shell::init_for_window(&win);
        gtk_layer_shell::set_monitor(&win, module_info.monitor);
        gtk_layer_shell::set_layer(&win, gtk_layer_shell::Layer::Overlay);
        gtk_layer_shell::set_namespace(&win, env!("CARGO_PKG_NAME"));

        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Top,
            if pos == BarPosition::Top { gap } else { 0 },
        );
        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Bottom,
            if pos == BarPosition::Bottom { gap } else { 0 },
        );
        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Left,
            if pos == BarPosition::Left { gap } else { 0 },
        );
        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Right,
            if pos == BarPosition::Right { gap } else { 0 },
        );

        gtk_layer_shell::set_anchor(
            &win,
            gtk_layer_shell::Edge::Top,
            pos == BarPosition::Top || orientation == Orientation::Vertical,
        );
        gtk_layer_shell::set_anchor(
            &win,
            gtk_layer_shell::Edge::Bottom,
            pos == BarPosition::Bottom,
        );
        gtk_layer_shell::set_anchor(
            &win,
            gtk_layer_shell::Edge::Left,
            pos == BarPosition::Left || orientation == Orientation::Horizontal,
        );
        gtk_layer_shell::set_anchor(
            &win,
            gtk_layer_shell::Edge::Right,
            pos == BarPosition::Right,
        );

        win.connect_leave_notify_event(move |win, ev| {
            const THRESHOLD: f64 = 3.0;

            let (w, h) = win.size();
            let (x, y) = ev.position();

            // some child widgets trigger this event
            // so check we're actually outside the window
            let hide = match pos {
                BarPosition::Top => {
                    x < THRESHOLD || y > f64::from(h) - THRESHOLD || x > f64::from(w) - THRESHOLD
                }
                BarPosition::Bottom => {
                    x < THRESHOLD || y < THRESHOLD || x > f64::from(w) - THRESHOLD
                }
                BarPosition::Left => {
                    y < THRESHOLD || x > f64::from(w) - THRESHOLD || y > f64::from(h) - THRESHOLD
                }
                BarPosition::Right => {
                    y < THRESHOLD || x < THRESHOLD || y > f64::from(h) - THRESHOLD
                }
            };

            if hide {
                win.hide();
            }

            Inhibit(false)
        });

        Self {
            window: win,
            cache: HashMap::new(),
            monitor: module_info.monitor.clone(),
            pos,
            current_widget: None,
        }
    }

    pub fn register_content(&mut self, key: usize, name: String, content: ModulePopupParts) {
        debug!("Registered popup content for #{}", key);

        for button in &content.buttons {
            let id = get_unique_usize();
            button.set_tag("popup-id", id);
        }

        self.cache.insert(key, (name, content));
    }

    pub fn show(&mut self, widget_id: usize, button_id: usize) {
        self.clear_window();

        if let Some((_name, content)) = self.cache.get(&widget_id) {
            self.current_widget = Some(widget_id);

            content.container.style_context().add_class("popup");
            self.window.add(&content.container);

            self.window.show();

            let button = content
                .buttons
                .iter()
                .find(|b| b.popup_id() == button_id)
                .expect("to find valid button");

            let orientation = self.pos.get_orientation();
            let geometry = button.geometry(orientation);

            self.set_pos(geometry);
        }
    }

    pub fn show_at(&self, widget_id: usize, geometry: WidgetGeometry) {
        self.clear_window();

        if let Some((_name, content)) = self.cache.get(&widget_id) {
            content.container.style_context().add_class("popup");
            self.window.add(&content.container);

            self.window.show();
            self.set_pos(geometry);
        }
    }

    fn clear_window(&self) {
        let children = self.window.children();
        for child in children {
            self.window.remove(&child);
        }
    }

    /// Hides the popover
    pub fn hide(&mut self) {
        self.current_widget = None;
        self.window.hide();
    }

    /// Checks if the popup is currently visible
    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn current_widget(&self) -> Option<usize> {
        self.current_widget
    }

    /// Sets the popup's X/Y position relative to the left or border of the screen
    /// (depending on orientation).
    fn set_pos(&self, geometry: WidgetGeometry) {
        let orientation = self.pos.get_orientation();

        let mon_workarea = self.monitor.workarea();
        let screen_size = if orientation == Orientation::Horizontal {
            mon_workarea.width()
        } else {
            mon_workarea.height()
        };

        let (popup_width, popup_height) = self.window.size();
        let popup_size = if orientation == Orientation::Horizontal {
            popup_width
        } else {
            popup_height
        };

        let widget_center = f64::from(geometry.position) + f64::from(geometry.size) / 2.0;

        let bar_offset = (f64::from(screen_size) - f64::from(geometry.bar_size)) / 2.0;

        let mut offset = bar_offset + (widget_center - (f64::from(popup_size) / 2.0)).round();

        if offset < 5.0 {
            offset = 5.0;
        } else if offset > f64::from(screen_size - popup_size) - 5.0 {
            offset = f64::from(screen_size - popup_size) - 5.0;
        }

        let edge = if orientation == Orientation::Horizontal {
            gtk_layer_shell::Edge::Left
        } else {
            gtk_layer_shell::Edge::Top
        };

        gtk_layer_shell::set_margin(&self.window, edge, offset as i32);
    }
}
