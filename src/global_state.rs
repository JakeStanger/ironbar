use crate::popup::Popup;
use crate::write_lock;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

/// Global application state shared across all bars.
///
/// Data that needs to be accessed from anywhere
/// that is not otherwise accessible should be placed on here.
#[derive(Debug)]
pub struct GlobalState {
    popups: HashMap<Box<str>, Arc<RwLock<Popup>>>,
}

impl GlobalState {
    pub(crate) fn new() -> Self {
        Self {
            popups: HashMap::new(),
        }
    }

    pub fn popups(&self) -> &HashMap<Box<str>, Arc<RwLock<Popup>>> {
        &self.popups
    }

    pub fn popups_mut(&mut self) -> &mut HashMap<Box<str>, Arc<RwLock<Popup>>> {
        &mut self.popups
    }

    pub fn with_popup_mut<F, T>(&self, monitor_name: &str, f: F) -> Option<T>
    where
        F: FnOnce(RwLockWriteGuard<Popup>) -> T,
    {
        let popup = self.popups().get(monitor_name);

        if let Some(popup) = popup {
            let popup = write_lock!(popup);
            Some(f(popup))
        } else {
            None
        }
    }
}
