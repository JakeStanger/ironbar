use glib::Propagation;
use gtk::gdk::Gravity;
use gtk::prelude::*;
use gtk::{EventBox, Image, Label, MenuItem};
use system_tray::item::{IconPixmap, StatusNotifierItem, Tooltip};

/// Main tray icon to show on the bar
pub(crate) struct TrayMenu {
    pub event_box: EventBox,
    pub widget: MenuItem,
    image_widget: Option<Image>,
    label_widget: Option<Label>,

    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_theme_path: Option<String>,
    pub icon_pixmap: Option<Vec<IconPixmap>>,
}

impl TrayMenu {
    pub fn new(item: StatusNotifierItem) -> Self {
        let event_box = EventBox::new();

        let widget = MenuItem::new();
        widget.style_context().add_class("item");
        event_box.add(&widget);

        event_box.show_all();

        Self {
            event_box,
            widget,
            image_widget: None,
            label_widget: None,
            title: item.title,
            icon_name: item.icon_name,
            icon_theme_path: item.icon_theme_path,
            icon_pixmap: item.icon_pixmap,
        }
    }

    /// Updates the label text, and shows it in favour of the image.
    pub fn set_label(&mut self, text: &str) {
        if let Some(image) = &self.image_widget {
            image.hide();
        }

        self.label_widget
            .get_or_insert_with(|| {
                let label = Label::new(None);
                self.widget.add(&label);
                label.show();
                label
            })
            .set_label(text);
    }

    /// Shows the label, using its current text.
    /// The image is hidden if present.
    pub fn show_label(&self) {
        if let Some(image) = &self.image_widget {
            image.hide();
        }

        if let Some(label) = &self.label_widget {
            label.show();
        }
    }

    /// Updates the image, and shows it in favour of the label.
    pub fn set_image(&mut self, image: &Image) {
        if let Some(label) = &self.label_widget {
            label.hide();
        }

        if let Some(old) = self.image_widget.replace(image.clone()) {
            self.widget.remove(&old);
        }

        self.widget.add(image);
        image.show();
    }

    pub fn label_widget(&self) -> Option<&Label> {
        self.label_widget.as_ref()
    }

    pub fn icon_name(&self) -> Option<&String> {
        self.icon_name.as_ref()
    }

    pub fn set_icon_name(&mut self, icon_name: Option<String>) {
        self.icon_name = icon_name;
    }

    pub fn set_tooltip(&self, tooltip: Option<Tooltip>) {
        let title = tooltip.map(|t| t.title);

        if let Some(widget) = &self.image_widget {
            widget.set_tooltip_text(title.as_deref());
        }

        if let Some(widget) = &self.label_widget {
            widget.set_tooltip_text(title.as_deref());
        }
    }

    pub fn set_menu_widget(&mut self, menu: system_tray::gtk_menu::Menu) {
        self.event_box
            .connect_button_press_event(move |event_box, _event| {
                menu.popup_at_widget(event_box, Gravity::North, Gravity::South, None);
                Propagation::Proceed
            });
    }
}
