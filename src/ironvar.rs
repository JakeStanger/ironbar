#![doc = include_str!("../docs/Ironvars.md")]

use crate::{arc_rw, send};
use color_eyre::{Report, Result};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

lazy_static! {
    static ref VARIABLE_MANAGER: Arc<RwLock<VariableManager>> = arc_rw!(VariableManager::new());
}

pub fn get_variable_manager() -> Arc<RwLock<VariableManager>> {
    VARIABLE_MANAGER.clone()
}

/// Global singleton manager for `IronVar` variables.
pub struct VariableManager {
    variables: HashMap<Box<str>, IronVar>,
}

impl VariableManager {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Sets the value for a variable,
    /// creating it if it does not exist.
    pub fn set(&mut self, key: Box<str>, value: String) -> Result<()> {
        if Self::key_is_valid(&key) {
            if let Some(var) = self.variables.get_mut(&key) {
                var.set(Some(value));
            } else {
                let var = IronVar::new(Some(value));
                self.variables.insert(key, var);
            }

            Ok(())
        } else {
            Err(Report::msg("Invalid key"))
        }
    }

    /// Gets the current value of an `ironvar`.
    /// Prefer to use `subscribe` where possible.
    pub fn get(&self, key: &str) -> Option<String> {
        self.variables.get(key).and_then(IronVar::get)
    }

    /// Subscribes to an `ironvar`, creating it if it does not exist.
    /// Any time the var is set, its value is sent on the channel.
    pub fn subscribe(&mut self, key: Box<str>) -> broadcast::Receiver<Option<String>> {
        self.variables
            .entry(key)
            .or_insert_with(|| IronVar::new(None))
            .subscribe()
    }

    fn key_is_valid(key: &str) -> bool {
        !key.is_empty()
            && key
                .chars()
                .all(|char| char.is_alphanumeric() || char == '_' || char == '-')
    }
}

/// Ironbar dynamic variable representation.
/// Interact with them through the `VARIABLE_MANAGER` `VariableManager` singleton.
#[derive(Debug)]
struct IronVar {
    value: Option<String>,
    tx: broadcast::Sender<Option<String>>,
    _rx: broadcast::Receiver<Option<String>>,
}

impl IronVar {
    /// Creates a new variable.
    fn new(value: Option<String>) -> Self {
        let (tx, rx) = broadcast::channel(32);

        Self { value, tx, _rx: rx }
    }

    /// Gets the current variable value.
    /// Prefer to subscribe to changes where possible.
    fn get(&self) -> Option<String> {
        self.value.clone()
    }

    /// Sets the current variable value.
    /// The change is broadcast to all receivers.
    fn set(&mut self, value: Option<String>) {
        self.value = value.clone();
        send!(self.tx, value);
    }

    /// Subscribes to the variable.
    /// The latest value is immediately sent to all receivers.
    fn subscribe(&self) -> broadcast::Receiver<Option<String>> {
        let rx = self.tx.subscribe();
        send!(self.tx, self.value.clone());
        rx
    }
}
