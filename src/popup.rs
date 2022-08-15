use crate::config::BarPosition;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Orientation};

#[derive(Debug, Clone)]
pub struct Popup {
    pub window: ApplicationWindow,
    pub container: gtk::Box,
    monitor: Monitor,
}

impl Popup {
    pub fn new(
        name: &str,
        app: &Application,
        monitor: &Monitor,
        orientation: Orientation,
        bar_position: &BarPosition,
    ) -> Self {
        let win = ApplicationWindow::builder().application(app).build();

        gtk_layer_shell::init_for_window(&win);
        gtk_layer_shell::set_layer(&win, gtk_layer_shell::Layer::Overlay);

        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Top,
            if bar_position == &BarPosition::Top {
                5
            } else {
                0
            },
        );
        gtk_layer_shell::set_margin(
            &win,
            gtk_layer_shell::Edge::Bottom,
            if bar_position == &BarPosition::Bottom {
                5
            } else {
                0
            },
        );
        gtk_layer_shell::set_margin(&win, gtk_layer_shell::Edge::Left, 0);
        gtk_layer_shell::set_margin(&win, gtk_layer_shell::Edge::Right, 0);

        gtk_layer_shell::set_anchor(
            &win,
            gtk_layer_shell::Edge::Top,
            bar_position == &BarPosition::Top,
        );
        gtk_layer_shell::set_anchor(
            &win,
            gtk_layer_shell::Edge::Bottom,
            bar_position == &BarPosition::Bottom,
        );
        gtk_layer_shell::set_anchor(&win, gtk_layer_shell::Edge::Left, true);
        gtk_layer_shell::set_anchor(&win, gtk_layer_shell::Edge::Right, false);

        let content = gtk::Box::builder()
            .orientation(orientation)
            .spacing(0)
            .hexpand(false)
            .name(name)
            .build();

        content.style_context().add_class("popup");

        win.add(&content);

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
            container: content,
            monitor: monitor.clone(),
        }
    }

    /// Shows the popover
    pub fn show(&self, button: &Button) {
        self.window.show_all();
        self.set_pos(button);
    }

    /// Hides the popover
    pub fn hide(&self) {
        self.window.hide();
    }

    /// Sets the popover's X position relative to the left border of the screen
    fn set_pos(&self, button: &Button) {
        let widget_width = button.allocation().width();
        let screen_width = self.monitor.workarea().width();
        let popup_width = self.window.allocated_width();

        let (widget_x, _) = button
            .translate_coordinates(&button.toplevel().unwrap(), 0, 0)
            .unwrap();

        let widget_center = f64::from(widget_x) + f64::from(widget_width) / 2.0;

        let mut offset = (widget_center - (f64::from(popup_width) / 2.0)).round();

        if offset < 5.0 {
            offset = 5.0;
        } else if offset > f64::from(screen_width - popup_width) - 5.0 {
            offset = f64::from(screen_width - popup_width) - 5.0;
        }

        gtk_layer_shell::set_margin(&self.window, gtk_layer_shell::Edge::Left, offset as i32);
    }
}
