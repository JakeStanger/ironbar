use crate::config::TruncateMode;
use glib::markup_escape_text;
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{Label, Orientation, Widget};

/// Represents a widget's size
/// and location relative to the bar's start edge.
#[derive(Debug, Copy, Clone)]
pub struct WidgetGeometry {
    /// Position of the start edge of the widget
    /// from the start edge of the bar.
    pub position: f64,
    /// The length of the widget.
    pub size: i32,
    /// The length of the bar.
    pub bar_size: i32,
}

pub trait IronbarGtkExt {
    /// Gets the geometry for the widget
    fn geometry(&self, orientation: Orientation) -> WidgetGeometry;

    /// Gets a data tag on a widget, if it exists.
    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V>;
    /// Sets a data tag on a widget.
    fn set_tag<V: 'static>(&self, key: &str, value: V);

    fn toplevel(&self) -> Widget;

    /// Returns an iterator for the widget's first-level children.
    fn children(&self) -> ChildIterator;
}

impl<W: IsA<Widget>> IronbarGtkExt for W {
    fn geometry(&self, orientation: Orientation) -> WidgetGeometry {
        let allocation = self.allocation();

        let widget_size = if orientation == Orientation::Horizontal {
            allocation.width()
        } else {
            allocation.height()
        };

        let top_level = self.toplevel();
        let top_level_allocation = top_level.allocation();

        let bar_size = if orientation == Orientation::Horizontal {
            top_level_allocation.width()
        } else {
            top_level_allocation.height()
        };

        let (widget_x, widget_y) = self
            .translate_coordinates(&top_level, 0.0, 0.0)
            .unwrap_or((0.0, 0.0));

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

    fn toplevel(&self) -> Widget {
        let mut curr = self.clone().upcast::<Widget>();
        let mut parent = curr.parent();

        while let Some(ref w) = parent {
            curr = w.clone();
            parent = w.parent();
        }

        curr
    }

    fn children(&self) -> ChildIterator {
        ChildIterator::new(self)
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
        let next = curr.as_ref().and_then(|w| w.next_sibling());
        self.curr = next.clone();
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
