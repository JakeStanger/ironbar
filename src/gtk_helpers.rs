use glib::IsA;
use gtk::prelude::*;
use gtk::Widget;

/// Adds a new CSS class to a widget.
pub fn add_class<W: IsA<Widget>>(widget: &W, class: &str) {
    widget.style_context().add_class(class);
}
