use std::collections::HashMap;

use crate::config::BarPosition;
use crate::modules::ModuleInfo;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct Popup {
    pub window: ApplicationWindow,
    pub cache: HashMap<usize, gtk::Box>,
    monitor: Monitor,
}

impl Popup {
    /// Creates a new popup window.
    /// This includes setting up gtk-layer-shell
    /// and an empty `gtk::Box` container.
    pub fn new(module_info: &ModuleInfo) -> Self {
        let pos = module_info.bar_position;
        let win = ApplicationWindow::builder()
            .application(module_info.app)
            .build();

        gtk_layer_shell::init_for_window(&win);
        gtk_layer_shell::set_layer(&win, gtk_layer_shell::Layer::Overlay);

        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Top,
            if pos == &BarPosition::Top { 5 } else { 0 },
        );
        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Bottom,
            if pos == &BarPosition::Bottom { 5 } else { 0 },
        );
        gtk_layer_shell::set_margin(&win, gtk_layer_shell::Edge::Left, 0);
        gtk_layer_shell::set_margin(&win, gtk_layer_shell::Edge::Right, 0);

        gtk_layer_shell::set_anchor(&win, gtk_layer_shell::Edge::Top, pos == &BarPosition::Top);
        gtk_layer_shell::set_anchor(
            &win,
            gtk_layer_shell::Edge::Bottom,
            pos == &BarPosition::Bottom,
        );
        gtk_layer_shell::set_anchor(&win, gtk_layer_shell::Edge::Left, true);
        gtk_layer_shell::set_anchor(&win, gtk_layer_shell::Edge::Right, false);

        win.connect_leave_notify_event(|win, ev| {
            const THRESHOLD: f64 = 3.0;

            let (w, _h) = win.size();
            let (x, y) = ev.position();

            // some child widgets trigger this event
            // so check we're actually outside the window
            if x < THRESHOLD || y < THRESHOLD || x > f64::from(w) - THRESHOLD {
                win.hide();
            }

            Inhibit(false)
        });

        Self {
            window: win,
            cache: HashMap::new(),
            monitor: module_info.monitor.clone(),
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

    /// Shows the popover
    pub fn show(&self, button_x: i32, button_width: i32) {
        self.window.show_all();
        self.set_pos(button_x, button_width);
    }

    /// Hides the popover
    pub fn hide(&self) {
        self.window.hide();
    }

    /// Checks if the popup is currently visible
    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    /// Sets the popover's X position relative to the left border of the screen
    fn set_pos(&self, button_x: i32, button_width: i32) {
        let screen_width = self.monitor.workarea().width();
        let popup_width = self.window.allocated_width();

        let widget_center = f64::from(button_x) + f64::from(button_width) / 2.0;

        let mut offset = (widget_center - (f64::from(popup_width) / 2.0)).round();

        if offset < 5.0 {
            offset = 5.0;
        } else if offset > f64::from(screen_width - popup_width) - 5.0 {
            offset = f64::from(screen_width - popup_width) - 5.0;
        }

        gtk_layer_shell::set_margin(&self.window, gtk_layer_shell::Edge::Left, offset as i32);
    }

    /// Gets the absolute X position of the button
    /// and its width.
    pub fn button_pos(button: &Button) -> (i32, i32) {
        let button_width = button.allocation().width();

        let top_level = button.toplevel().expect("Failed to get top-level widget");
        let (button_x, _) = button
            .translate_coordinates(&top_level, 0, 0)
            .unwrap_or((0, 0));

        (button_x, button_width)
    }
}
