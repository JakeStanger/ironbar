use crate::popup::Popup;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

/// Global application state shared across all bars.
///
/// Data that needs to be accessed from anywhere
/// that is not otherwise accessible should be placed on here.
#[derive(Debug)]
pub struct GlobalState {
    popups: HashMap<Box<str>, Rc<RefCell<Popup>>>,
}

impl GlobalState {
    pub(crate) fn new() -> Self {
        Self {
            popups: HashMap::new(),
        }
    }

    pub fn popups(&self) -> &HashMap<Box<str>, Rc<RefCell<Popup>>> {
        &self.popups
    }

    pub fn popups_mut(&mut self) -> &mut HashMap<Box<str>, Rc<RefCell<Popup>>> {
        &mut self.popups
    }

    pub fn with_popup_mut<F, T>(&self, monitor_name: &str, f: F) -> Option<T>
    where
        F: FnOnce(RefMut<Popup>) -> T,
    {
        let popup = self.popups().get(monitor_name);

        if let Some(popup) = popup {
            let popup = popup.borrow_mut();
            Some(f(popup))
        } else {
            None
        }
    }
}
