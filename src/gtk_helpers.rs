use crate::config::{MarqueeMode, TruncateMode};
use glib::{IsA, markup_escape_text};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{Label, Orientation, ScrolledWindow, TickCallbackId, Widget};

/// Represents a widget's size
/// and location relative to the bar's start edge.
#[derive(Debug, Copy, Clone)]
pub struct WidgetGeometry {
    /// Position of the start edge of the widget
    /// from the start edge of the bar.
    pub position: i32,
    /// The length of the widget.
    pub size: i32,
    /// The length of the bar.
    pub bar_size: i32,
}

pub trait IronbarGtkExt {
    /// Adds a new CSS class to the widget.
    fn add_class(&self, class: &str);
    /// Removes a CSS class from the widget
    fn remove_class(&self, class: &str);
    /// Gets the geometry for the widget
    fn geometry(&self, orientation: Orientation) -> WidgetGeometry;

    /// Gets a data tag on a widget, if it exists.
    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V>;
    /// Sets a data tag on a widget.
    fn set_tag<V: 'static>(&self, key: &str, value: V);
}

impl<W: IsA<Widget>> IronbarGtkExt for W {
    fn add_class(&self, class: &str) {
        self.style_context().add_class(class);
    }

    fn remove_class(&self, class: &str) {
        self.style_context().remove_class(class);
    }

    fn geometry(&self, orientation: Orientation) -> WidgetGeometry {
        let allocation = self.allocation();

        let widget_size = if orientation == Orientation::Horizontal {
            allocation.width()
        } else {
            allocation.height()
        };

        let top_level = self.toplevel().expect("Failed to get top-level widget");
        let top_level_allocation = top_level.allocation();

        let bar_size = if orientation == Orientation::Horizontal {
            top_level_allocation.width()
        } else {
            top_level_allocation.height()
        };

        let (widget_x, widget_y) = self
            .translate_coordinates(&top_level, 0, 0)
            .unwrap_or((0, 0));

        let widget_pos = if orientation == Orientation::Horizontal {
            widget_x
        } else {
            widget_y
        };

        WidgetGeometry {
            position: widget_pos,
            size: widget_size,
            bar_size,
        }
    }

    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V> {
        unsafe { self.data(key).map(|val| val.as_ref()) }
    }

    fn set_tag<V: 'static>(&self, key: &str, value: V) {
        unsafe { self.set_data(key, value) }
    }
}

pub trait IronbarLabelExt {
    /// Sets the label value to the provided string.
    ///
    /// If the label does not contain markup `span` tags,
    /// the text is escaped to avoid issues with special characters (ie `&`).
    /// Otherwise, the text is used verbatim, and it is up to the user to escape.
    fn set_label_escaped(&self, label: &str);

    fn truncate(&self, mode: TruncateMode);
}

impl IronbarLabelExt for Label {
    fn set_label_escaped(&self, label: &str) {
        if label.contains("<span") {
            self.set_label(label);
        } else {
            self.set_label(&markup_escape_text(label));
        }
    }

    fn truncate(&self, mode: TruncateMode) {
        self.set_ellipsize(<TruncateMode as Into<EllipsizeMode>>::into(mode));

        if let Some(length) = mode.length() {
            self.set_width_chars(length);
        }

        if let Some(length) = mode.max_length() {
            self.set_max_width_chars(length);
        }
    }
}

// Calculate pixel width of a string given the label it's displayed in
fn pixel_width(label: &gtk::Label, string: &str) -> i32 {
    let layout = label.create_pango_layout(Some(string));
    let (w, _) = layout.size(); // in Pango units (1/1024 px)
    w / gtk::pango::SCALE // back to integer pixels
}

pub fn create_marquee_widget(
    label: &Label,
    text: &str,
    marquee_mode: MarqueeMode,
) -> ScrolledWindow {
    let MarqueeMode {
        max_length,
        pause_on_hover,
        pause_on_hover_invert,
        ..
    } = marquee_mode;

    let scrolled = ScrolledWindow::builder()
        .vscrollbar_policy(gtk::PolicyType::Never)
        .build();

    if let Some(max_length) = max_length {
        let sample_string = text.chars().take(max_length as usize).collect::<String>();
        let width = pixel_width(label, &sample_string);
        scrolled.set_min_content_width(width);
    }

    scrolled.add(label);

    // Set initial state.
    // The size_allocate signal will handle the rest.
    label.set_label(text);

    let label = label.clone();
    let text = text.to_string();
    let sep = "    ".to_string();
    let ease_pause = std::time::Duration::from_secs(5);

    // Use a RefCell to hold the tick_id to allow mutation from the closure
    let tick_id = std::rc::Rc::new(std::cell::RefCell::new(None::<TickCallbackId>));
    let is_hovered = std::rc::Rc::new(std::cell::RefCell::new(false));
    let pause_started_at = std::rc::Rc::new(std::cell::RefCell::new(None::<std::time::Instant>));

    let tick_id_clone = tick_id.clone();
    let is_hovered_clone = is_hovered.clone();
    scrolled.connect_size_allocate(move |scrolled, _| {
        let allocated_width = scrolled.allocation().width();
        let original_text_width = pixel_width(&label, &text);

        let is_scrolling = tick_id_clone.borrow().is_some();

        if original_text_width > allocated_width {
            // Needs to scroll
            if !is_scrolling {
                let duplicated_text = format!("{}{}{}", &text, &sep, &text);
                label.set_label(&duplicated_text);

                let reset_at = pixel_width(&label, &format!("{}{}", &text, &sep)) as f64;

                let is_hovered_clone_tick = is_hovered_clone.clone();
                let pause_started_at_clone = pause_started_at.clone();
                let id = scrolled.add_tick_callback(move |widget, _| {
                    let is_paused = if let Some(start_time) = *pause_started_at_clone.borrow() {
                        start_time.elapsed() <= ease_pause
                    } else {
                        false
                    };

                    if is_paused {
                        return glib::ControlFlow::Continue;
                    }

                    // check if we need to resume
                    if pause_started_at_clone.borrow().is_some() {
                        *pause_started_at_clone.borrow_mut() = None;
                    }

                    let should_scroll = if pause_on_hover_invert {
                        *is_hovered_clone_tick.borrow()
                    } else if pause_on_hover {
                        !*is_hovered_clone_tick.borrow()
                    } else {
                        true
                    };

                    if should_scroll {
                        let hadjustment = widget.hadjustment();
                        let v = hadjustment.value() + 0.5;
                        if v >= reset_at {
                            hadjustment.set_value(v - reset_at);
                            *pause_started_at_clone.borrow_mut() = Some(std::time::Instant::now());
                        } else {
                            hadjustment.set_value(v);
                        }
                    }
                    glib::ControlFlow::Continue
                });

                *tick_id_clone.borrow_mut() = Some(id);
            }
        } else {
            // No need to scroll
            if is_scrolling {
                if let Some(id) = tick_id_clone.borrow_mut().take() {
                    id.remove();
                }
                label.set_label(&text);
            }
        }
    });

    if pause_on_hover || pause_on_hover_invert {
        let is_hovered_enter = is_hovered.clone();
        scrolled.connect_enter_notify_event(move |_, _| {
            *is_hovered_enter.borrow_mut() = true;
            glib::Propagation::Stop
        });

        scrolled.connect_leave_notify_event(move |_, _| {
            *is_hovered.borrow_mut() = false;
            glib::Propagation::Stop
        });
    }

    scrolled
}
