use crate::config::BarPosition;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Orientation};

#[derive(Debug, Clone)]
pub struct Popup {
    pub window: ApplicationWindow,
    pub container: gtk::Box,
}

pub enum PopupAlignment {
    Left,
    Center,
    Right,
}

impl Popup {
    pub fn new(
        name: &str,
        app: &Application,
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
            let (w, _h) = win.size();
            let (x, y) = ev.position();

            const THRESHOLD: f64 = 3.0;

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
        }
    }

    /// Sets the popover's X position relative to the left border of the screen
    pub fn set_pos(&self, pos: f64, alignment: PopupAlignment) {
        let width = self.window.allocated_width();

        let offset = match alignment {
            PopupAlignment::Left => pos,
            PopupAlignment::Center => (pos - (f64::from(width) / 2.0)).round(),
            PopupAlignment::Right => pos - f64::from(width),
        };

        gtk_layer_shell::set_margin(&self.window, gtk_layer_shell::Edge::Left, offset as i32);
    }

    /// Shows the popover
    pub fn show(&self) {
        self.window.show_all();
    }

    /// Hides the popover
    pub fn hide(&self) {
        self.window.hide();
    }
}
