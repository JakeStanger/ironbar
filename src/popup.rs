use crate::config::BarPosition;
use crate::modules::{ModuleInfo, ModulePopupParts, PopupButton};
use crate::rc_mut;
use gtk::prelude::*;
use gtk::{Button, Popover, PositionType};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use tracing::{debug, error};

#[derive(Debug)]
pub struct PopupCacheValue {
    // pub name: String,
    pub content: gtk::Box,
}

#[derive(Debug, Clone, Copy)]
struct CurrentWidgetInfo {
    widget_id: usize,
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
    pub fn new(module_info: &ModuleInfo, gap: i32, autohide: bool) -> Self {
        let pos = module_info.bar_position;

        let position = match pos {
            BarPosition::Top => PositionType::Bottom,
            BarPosition::Bottom => PositionType::Top,
            BarPosition::Left => PositionType::Right,
            BarPosition::Right => PositionType::Left,
        };

        let (offset_x, offset_y) = match pos {
            BarPosition::Top => (0, gap),
            BarPosition::Bottom => (0, -gap),
            BarPosition::Left | BarPosition::Right => (gap, 0),
        };

        let popover = Popover::builder()
            .has_arrow(false)
            .autohide(autohide)
            .position(position)
            .build();

        popover.set_offset(offset_x, offset_y);

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

    pub fn register_content(&self, key: usize, content: ModulePopupParts) {
        debug!("Registered popup content for #{}", key);

        for button in &content.buttons {
            button.ensure_popup_id();
        }

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
        }
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
            *self.current_widget.borrow_mut() = Some(CurrentWidgetInfo { widget_id });

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

            content.add_css_class("popup");
            self.popover.set_child(Some(content));
            self.popover.unparent();
            self.popover.set_parent(&button);

            println!("popup");
            self.popover.popup();
        }
    }

    /// Attempts to show the popup, parented on a particular button.
    /// This relies on the `widget_id` existing in the cache,
    /// otherwise this is a no-op.
    ///
    /// Returns whether the popup is shown.
    pub fn show_for(&self, widget_id: usize, button: &Button) -> bool {
        self.clear_window();

        if let Some(PopupCacheValue { content, .. }) = self.container_cache.borrow().get(&widget_id)
        {
            *self.current_widget.borrow_mut() = Some(CurrentWidgetInfo { widget_id });

            content.add_css_class("popup");
            self.popover.set_child(Some(content));
            self.popover.unparent();
            self.popover.set_parent(button);
            self.popover.popup();

            true
        } else {
            false
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
