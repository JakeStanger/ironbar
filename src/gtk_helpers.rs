use crate::config::{MarqueeMode, MarqueeOnHover, TruncateMode};
use glib::ControlFlow;
use glib::{SignalHandlerId, markup_escape_text};
use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY, BUTTON_SECONDARY, Paintable};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{
    EventControllerMotion, EventSequenceState, GestureClick, Label, ScrolledWindow, Snapshot,
    Widget,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum MouseButton {
    Any,
    Primary = BUTTON_PRIMARY,
    Middle = BUTTON_MIDDLE,
    Secondary = BUTTON_SECONDARY,
}

pub trait IronbarGtkExt {
    /// Gets a data tag on a widget, if it exists.
    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V>;
    /// Sets a data tag on a widget.
    fn set_tag<V: 'static>(&self, key: &str, value: V);

    /// Returns an iterator for the widget's first-level children.
    fn children(&self) -> ChildIterator;

    /// Adds a `GestureClick` controller with a `connect_pressed` signal callback.
    /// A mouse button can be specified to filter click events.
    fn connect_pressed<F>(&self, button: MouseButton, f: F) -> SignalHandlerId
    where
        F: Fn() + 'static;
}

impl<W: IsA<Widget>> IronbarGtkExt for W {
    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V> {
        unsafe { self.data(key).map(|val| val.as_ref()) }
    }

    fn set_tag<V: 'static>(&self, key: &str, value: V) {
        unsafe { self.set_data(key, value) }
    }

    fn children(&self) -> ChildIterator {
        ChildIterator::new(self)
    }

    fn connect_pressed<F>(&self, button: MouseButton, f: F) -> SignalHandlerId
    where
        F: Fn() + 'static,
    {
        let controller = GestureClick::new();

        if button != MouseButton::Any {
            controller.set_button(button as u32);
        }

        let id = controller.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(EventSequenceState::Claimed);
            f();
        });

        self.add_controller(controller);
        id
    }
}

pub struct ChildIterator {
    curr: Option<Widget>,
}

impl ChildIterator {
    fn new<W: IsA<Widget>>(parent: &W) -> Self {
        Self {
            curr: parent.first_child(),
        }
    }
}

impl Iterator for ChildIterator {
    type Item = Widget;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr.clone();
        let next = curr.as_ref().and_then(WidgetExt::next_sibling);
        self.curr.clone_from(&next);
        curr // return current rather than next to include first child
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

pub trait IronbarPaintableExt {
    /// Scales a `Paintable`. to the requested size,
    /// returning a new `Paintable`.
    ///
    /// Aspect ratio is preserved.
    fn scale(self, target_width: f64, target_height: f64) -> Option<Paintable>;
}

impl<T> IronbarPaintableExt for T
where
    T: IsA<Paintable>,
{
    fn scale(self, target_width: f64, target_height: f64) -> Option<Paintable> {
        let ratio = self.intrinsic_aspect_ratio();

        let (width, height) = if ratio > 1.0 {
            (target_width, target_width / ratio)
        } else {
            (target_height * ratio, target_height)
        };

        let snapshot = Snapshot::new();
        self.snapshot(&snapshot, width, height);
        snapshot.to_paintable(None)
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
    // Default constants
    const DEFAULT_SCROLL_SPEED: f64 = 0.5; // pixels per tick
    const DEFAULT_PAUSE_DURATION_MS: u64 = 5000; // 5 seconds
    const DEFAULT_SEPARATOR: &str = "    "; // 4 spaces

    let MarqueeMode {
        max_length,
        scroll_speed,
        pause_duration,
        separator,
        on_hover,
        ..
    } = marquee_mode;

    let scroll_speed = scroll_speed.unwrap_or(DEFAULT_SCROLL_SPEED);
    let pause_duration_ms = pause_duration.unwrap_or(DEFAULT_PAUSE_DURATION_MS);
    let sep = separator.unwrap_or_else(|| DEFAULT_SEPARATOR.to_string());
    let ease_pause = Duration::from_millis(pause_duration_ms);

    let scrolled = ScrolledWindow::builder()
        .vscrollbar_policy(gtk::PolicyType::Never)
        .build();

    scrolled.hscrollbar().set_visible(false);

    // Set `min-width` to the pixel width of the text, but not wider than `max_length` (as calculated)
    if let Some(max_length) = max_length {
        let sample_string = text.chars().take(max_length as usize).collect::<String>();
        let width = pixel_width(label, &sample_string);
        scrolled.set_min_content_width(width);
    }

    scrolled.set_child(Some(label));

    // Set initial state
    label.set_label(text);

    let label = label.clone();
    let text = text.to_string();

    // Cache the original text width (calculated once upfront)
    let original_text_width = pixel_width(&label, &text);

    let is_hovered = Rc::new(RefCell::new(false));
    let pause_started_at = Rc::new(RefCell::new(None::<Instant>));
    let is_scrolling = Rc::new(RefCell::new(false));
    let reset_at_cached = Rc::new(RefCell::new(None::<f64>));

    // Start a tick callback that checks size and scrolls if needed
    let is_hovered_clone = is_hovered.clone();
    let pause_started_at_clone = pause_started_at.clone();
    let is_scrolling_clone = is_scrolling.clone();
    let reset_at_cached_clone = reset_at_cached.clone();
    scrolled.add_tick_callback(move |widget, _| {
        let allocated_width = widget.width();

        // Check if we need to scroll based on text width vs allocated width
        let needs_scroll = original_text_width > allocated_width;

        if needs_scroll {
            // Setup scrolling if not already set up
            if !*is_scrolling_clone.borrow() {
                let duplicated_text = format!("{}{}{}", &text, &sep, &text);
                label.set_label(&duplicated_text);

                // Calculate and cache reset position (where to loop back to)
                let reset_at = pixel_width(&label, &format!("{}{}", &text, &sep)) as f64;
                *reset_at_cached_clone.borrow_mut() = Some(reset_at);

                *is_scrolling_clone.borrow_mut() = true;
            }

            // Use cached reset position
            let reset_at = reset_at_cached_clone.borrow().unwrap();

            // Check if paused
            let is_paused = if let Some(start_time) = *pause_started_at_clone.borrow() {
                start_time.elapsed() <= ease_pause
            } else {
                false
            };

            if is_paused {
                return ControlFlow::Continue;
            }

            // Check if we need to resume
            if pause_started_at_clone.borrow().is_some() {
                *pause_started_at_clone.borrow_mut() = None;
            }

            // Determine if we should scroll based on hover state
            let should_scroll = match on_hover {
                MarqueeOnHover::Play => *is_hovered_clone.borrow(),
                MarqueeOnHover::Pause => !*is_hovered_clone.borrow(),
                MarqueeOnHover::None => true,
            };

            if should_scroll {
                let hadjustment = widget.hadjustment();
                let v = hadjustment.value() + scroll_speed;
                if v >= reset_at {
                    hadjustment.set_value(v - reset_at);
                    *pause_started_at_clone.borrow_mut() = Some(Instant::now());
                } else {
                    hadjustment.set_value(v);
                }
            }
        } else {
            // No need to scroll - reset if currently scrolling
            if *is_scrolling_clone.borrow() {
                label.set_label(&text);
                widget.hadjustment().set_value(0.0);
                *is_scrolling_clone.borrow_mut() = false;
                *reset_at_cached_clone.borrow_mut() = None; // Clear cache
            }
        }

        ControlFlow::Continue
    });

    if on_hover != MarqueeOnHover::None {
        let motion_controller = EventControllerMotion::new();

        let is_hovered_enter = is_hovered.clone();
        motion_controller.connect_enter(move |_, _, _| {
            *is_hovered_enter.borrow_mut() = true;
        });

        let is_hovered_leave = is_hovered.clone();
        motion_controller.connect_leave(move |_| {
            *is_hovered_leave.borrow_mut() = false;
        });

        scrolled.add_controller(motion_controller);
    }

    scrolled
}
