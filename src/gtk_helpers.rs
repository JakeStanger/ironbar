use crate::config::TruncateMode;
use glib::{SignalHandlerId, markup_escape_text};
use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY, BUTTON_SECONDARY};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{EventSequenceState, GestureClick, Label, Widget};

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
