use glib::IsA;
use gtk::prelude::*;
use gtk::{Orientation, Widget};

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
    /// Adds a new CSS class to the widget.
    fn add_class(&self, class: &str);
    /// Gets the geometry for the widget
    fn geometry(&self, orientation: Orientation) -> WidgetGeometry;

    /// Gets a data tag on a widget, if it exists.
    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V>;
    /// Sets a data tag on a widget.
    fn set_tag<V: 'static>(&self, key: &str, value: V);

    fn children(&self) -> Vec<Box<dyn AsRef<Widget>>>;
}

impl<W: IsA<Widget>> IronbarGtkExt for W {
    fn add_class(&self, class: &str) {
        self.style_context().add_class(class);
    }

    fn geometry(&self, orientation: Orientation) -> WidgetGeometry {
        let allocation = self.allocation();

        let widget_size = if orientation == Orientation::Horizontal {
            allocation.width()
        } else {
            allocation.height()
        };

        let top_level = self.root().expect("Failed to get root widget");
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

    fn children(&self) -> Vec<Box<dyn AsRef<Widget>>> {
        let mut widget = self.first_child();
        let mut children = vec![];

        while let Some(w) = widget {
            children.push(Box::new(w));
            widget = w.next_sibling();
        }

        children
    }
}

// struct IteratorWrapper<W: IsA<Widget>>(W);
//
// impl<W> Iterator for IteratorWrapper<W> {
//     type Item = Box<dyn AsRef<Widget>>;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         self.0
//     }
// }
//
// struct IntoIter<W: IsA<Widget>> {
//     widget: W,
//     next: Option<Box<dyn AsRef<Widget>>>
// }
//
// impl<W: IsA<Widget>> IntoIterator for IteratorWrapper<W> {
//     type Item = Box<dyn AsRef<Widget>>;
//     type IntoIter = IntoIter<Self>;
//
//     fn into_iter(self) -> Self::IntoIter {
//         IntoIter {
//             widget: self,
//             next: self.first_child().map(Box::new)
//         }
//     }
// }
