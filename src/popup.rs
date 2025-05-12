use crate::clients::wayland::{OutputEvent, OutputEventType};
use crate::config::BarPosition;
use crate::gtk_helpers::{IronbarGtkExt, WidgetGeometry};
use crate::modules::{ModuleInfo, ModulePopupParts, PopupButton};
use crate::{Ironbar, glib_recv, rc_mut};
use glib::ffi::gpointer;
use glib::translate::ToGlibPtr;
use glib::{Object, Propagation, clone};
use gtk::ffi::{GtkLabel, GtkWidget, GtkWidgetPrivate};
use gtk::gdk::Rectangle;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Button, EventControllerMotion, Orientation, Popover, PositionType, Widget,
    Window,
};
use gtk_layer_shell::{Edge, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use tracing::{debug, error, trace};

#[derive(Debug)]
pub struct PopupCacheValue {
    // pub name: String,
    pub content: gtk::Box,
}

#[derive(Debug, Clone, Copy)]
struct CurrentWidgetInfo {
    widget_id: usize,
    button_id: usize,
}

pub type ButtonFinder = dyn Fn(usize) -> Option<Button> + 'static;

#[derive(Clone)]
pub struct Popup {
    pub popover: Popover,
    pub container_cache: Rc<RefCell<HashMap<usize, PopupCacheValue>>>,
    pub button_finder_cache: Rc<RefCell<HashMap<usize, Box<ButtonFinder>>>>,
    pub button_cache: Rc<RefCell<Vec<Button>>>,
    pos: BarPosition,
    current_widget: Rc<RefCell<Option<CurrentWidgetInfo>>>,
}

impl Debug for Popup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popup")
            .field("popover", &self.popover)
            .field("container_cache", &self.container_cache)
            .field("button_finder_cache", &"<callbacks>")
            .field("button_cache", &self.button_cache)
            .field("pos", &self.pos)
            .field("current_widget", &self.current_widget)
            .finish()
    }
}

impl Popup {
    /// Creates a new popup window.
    /// This includes setting up gtk-layer-shell
    /// and an empty `gtk::Box` container.
    pub fn new(
        ironbar: Rc<Ironbar>,
        module_info: &ModuleInfo,
        output_size: (i32, i32),
        gap: i32,
    ) -> Self {
        let pos = module_info.bar_position;
        let orientation = pos.orientation();

        let position = match pos {
            BarPosition::Top => PositionType::Bottom,
            BarPosition::Bottom => PositionType::Top,
            BarPosition::Left => PositionType::Right,
            BarPosition::Right => PositionType::Left,
        };

        let popover = Popover::builder()
            .has_arrow(false)
            .autohide(false) // enabling autohide forces kb/m grab which we don't want
            .position(position)
            .build();

        popover.connect_closed(|popover| {
            popover.unparent();
        });

        Self {
            popover,
            container_cache: rc_mut!(HashMap::new()),
            button_cache: rc_mut!(vec![]),
            button_finder_cache: rc_mut!(HashMap::new()),
            pos,
            current_widget: rc_mut!(None),
        }
    }

    pub fn register_content(&self, key: usize, name: String, content: ModulePopupParts) {
        debug!("Registered popup content for #{}", key);

        for button in &content.buttons {
            button.ensure_popup_id();
        }

        let cache = self.container_cache.clone();
        let button_cache = self.button_cache.clone();

        self.button_cache
            .borrow_mut()
            .append(&mut content.buttons.clone());

        self.container_cache.borrow_mut().insert(
            key,
            PopupCacheValue {
                content: content.container.clone(),
            },
        );

        if let Some(button_finder) = content.button_finder {
            self.button_finder_cache
                .borrow_mut()
                .insert(key, Box::new(button_finder));
        };
    }

    pub fn register_button(&self, button: Button) {
        button.ensure_popup_id();
        self.button_cache.borrow_mut().push(button);
    }

    pub fn register_button_finder(&self, key: usize, finder: Box<ButtonFinder>) {
        self.button_finder_cache
            .borrow_mut()
            .insert(key, Box::new(finder));
    }

    pub fn unregister_button(&self, button: &Button) {
        self.button_cache.borrow_mut().retain(|b| b != button);
    }

    pub fn show(&self, widget_id: usize, button_id: usize) {
        self.clear_window();

        if let Some(PopupCacheValue { content, .. }) = self.container_cache.borrow().get(&widget_id)
        {
            *self.current_widget.borrow_mut() = Some(CurrentWidgetInfo {
                widget_id,
                button_id,
            });

            let button = if let Some(finder) = self.button_finder_cache.borrow().get(&widget_id) {
                finder(button_id)
            } else {
                let button_cache = self.button_cache.borrow();
                button_cache
                    .iter()
                    .find(|b| b.popup_id() == button_id)
                    .cloned()
            };

            let Some(button) = button else {
                error!("Could not find button for popup");
                return;
            };

            content.add_class("popup");
            self.popover.set_child(Some(content));
            self.popover.unparent();
            self.popover.set_parent(&button);
            self.popover.popup();
        }
    }

    fn clear_window(&self) {
        self.popover.set_child(None::<&gtk::Box>);
    }

    /// Hides the popup
    pub fn hide(&self) {
        *self.current_widget.borrow_mut() = None;
        self.popover.popdown();
        self.popover.unparent();
    }

    /// Checks if the popup is currently visible
    pub fn visible(&self) -> bool {
        self.popover.is_visible()
    }

    pub fn current_widget(&self) -> Option<usize> {
        self.current_widget.borrow().map(|w| w.widget_id)
    }
}
