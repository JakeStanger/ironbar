use std::collections::HashMap;

use crate::config::BarPosition;
use crate::modules::ModuleInfo;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, Orientation};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct Popup {
    pub window: ApplicationWindow,
    pub cache: HashMap<usize, gtk::Box>,
    monitor: Monitor,
    pos: BarPosition,
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
        }
    }

    pub fn register_content(&mut self, key: usize, content: gtk::Box) {
        debug!("Registered popup content for #{}", key);
        self.cache.insert(key, content);
    }

    pub fn show_content(&self, key: usize) {
        self.clear_window();

        if let Some(content) = self.cache.get(&key) {
            content.style_context().add_class("popup");
            self.window.add(content);
        }
    }

    fn clear_window(&self) {
        let children = self.window.children();
        for child in children {
            self.window.remove(&child);
        }
    }

    /// Shows the popup
    pub fn show(&self, geometry: ButtonGeometry) {
        self.window.show();
        self.set_pos(geometry);
    }

    /// Hides the popover
    pub fn hide(&self) {
        self.window.hide();
    }

    /// Checks if the popup is currently visible
    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    /// Sets the popup's X/Y position relative to the left or border of the screen
    /// (depending on orientation).
    fn set_pos(&self, geometry: ButtonGeometry) {
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

    /// Gets the absolute X position of the button
    /// and its width / height (depending on orientation).
    pub fn button_pos(button: &Button, orientation: Orientation) -> ButtonGeometry {
        let button_size = if orientation == Orientation::Horizontal {
            button.allocation().width()
        } else {
            button.allocation().height()
        };

        let top_level = button.toplevel().expect("Failed to get top-level widget");

        let bar_size = if orientation == Orientation::Horizontal {
            top_level.allocation().width()
        } else {
            top_level.allocation().height()
        };

        let (button_x, button_y) = button
            .translate_coordinates(&top_level, 0, 0)
            .unwrap_or((0, 0));

        let button_pos = if orientation == Orientation::Horizontal {
            button_x
        } else {
            button_y
        };

        ButtonGeometry {
            position: button_pos,
            size: button_size,
            bar_size,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ButtonGeometry {
    position: i32,
    size: i32,
    bar_size: i32,
}
