use crate::config::TruncateMode;
use glib::{SignalHandlerId, markup_escape_text};
use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY, BUTTON_SECONDARY, Paintable};
use gtk::glib;
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{EventSequenceState, GestureClick, Label, Snapshot, Widget};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

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

    /// Adds a `GestureClick` controller with separate handlers for single and double clicks.
    /// Single-click is delayed by the double-click timeout (250ms) to distinguish from double-clicks.
    /// If `on_double` is None, behaves like `connect_pressed` with no delay.
    fn connect_pressed_with_double_click<F1, F2>(
        &self,
        button: MouseButton,
        on_single: F1,
        on_double: Option<F2>,
    ) -> SignalHandlerId
    where
        F1: Fn() + 'static,
        F2: Fn() + 'static;
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

    fn connect_pressed_with_double_click<F1, F2>(
        &self,
        button: MouseButton,
        on_single: F1,
        on_double: Option<F2>,
    ) -> SignalHandlerId
    where
        F1: Fn() + 'static,
        F2: Fn() + 'static,
    {
        let controller = GestureClick::new();

        if button != MouseButton::Any {
            controller.set_button(button as u32);
        }

        // If no double-click handler provided, behave like regular connect_pressed
        let Some(on_double) = on_double else {
            let id = controller.connect_pressed(move |gesture, _, _, _| {
                gesture.set_state(EventSequenceState::Claimed);
                on_single();
            });
            self.add_controller(controller);
            return id;
        };

        // Wrap callbacks in Rc to make them cloneable
        let on_single = Rc::new(on_single);
        let on_double = Rc::new(on_double);

        // Track pending single-click timeout to cancel if double-click occurs
        let pending_single_click: Rc<Cell<Option<glib::SourceId>>> = Rc::new(Cell::new(None));

        let id = controller.connect_pressed(move |gesture, n_press, _, _| {
            gesture.set_state(EventSequenceState::Claimed);

            match n_press {
                1 => {
                    // Single click: delay action to see if double-click comes
                    let on_single = on_single.clone();
                    let pending = pending_single_click.clone();

                    // Cancel any existing pending single-click
                    if let Some(source_id) = pending.take() {
                        source_id.remove();
                    }

                    // Schedule single-click action after double-click timeout
                    let timeout_ms = crate::config::get_double_click_time_ms();
                    let source_id = glib::timeout_add_local_once(
                        Duration::from_millis(timeout_ms),
                        move || {
                            on_single();
                            pending.set(None);
                        },
                    );

                    pending_single_click.set(Some(source_id));
                }
                2 => {
                    // Double click: cancel pending single-click and execute double-click action
                    if let Some(source_id) = pending_single_click.take() {
                        source_id.remove();
                    }

                    on_double();
                }
                _ => {
                    // Ignore triple-clicks and beyond
                }
            }
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
