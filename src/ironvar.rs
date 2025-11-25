#![doc = include_str!("../docs/Ironvars.md")]

use crate::channels::SyncSenderExt;
use crate::{arc_rw, read_lock, write_lock};
use miette::{Report, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

pub type NamespaceTrait = Arc<dyn Namespace + Sync + Send>;

pub trait Namespace {
    fn get(&self, key: &str) -> Option<String>;
    fn list(&self) -> Vec<String>;

    fn get_all(&self) -> HashMap<Box<str>, String> {
        self.list()
            .into_iter()
            .filter_map(|name| self.get(&name).map(|value| (name.into(), value)))
            .collect()
    }

    fn namespaces(&self) -> Vec<String>;
    fn get_namespace(&self, key: &str) -> Option<NamespaceTrait>;
}

pub trait WritableNamespace: Namespace {
    fn set(&self, key: &str, value: String) -> Result<()>;
}

/// Global singleton manager for `IronVar` variables.
pub struct VariableManager {
    variables: Arc<RwLock<HashMap<Box<str>, IronVar>>>,
    namespaces: Arc<RwLock<HashMap<Box<str>, NamespaceTrait>>>,
}

impl Default for VariableManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VariableManager {
    pub fn new() -> Self {
        Self {
            variables: arc_rw!(HashMap::new()),
            namespaces: arc_rw!(HashMap::new()),
        }
    }

    /// Subscribes to an `ironvar`, creating it if it does not exist.
    /// Any time the var is set, its value is sent on the channel.
    pub fn subscribe(&self, key: Box<str>) -> broadcast::Receiver<Option<String>> {
        write_lock!(self.variables)
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

    pub fn register_namespace<N>(&self, name: &str, namespace: Arc<N>)
    where
        N: Namespace + Sync + Send + 'static,
    {
        write_lock!(self.namespaces).insert(name.into(), namespace);
    }
}

impl Namespace for VariableManager {
    fn get(&self, key: &str) -> Option<String> {
        if key.contains('.') {
            let (ns, key) = key.split_once('.')?;

            let namespaces = read_lock!(self.namespaces);
            let ns = namespaces.get(ns)?;

            ns.get(key).as_deref().map(ToOwned::to_owned)
        } else {
            read_lock!(self.variables).get(key).and_then(IronVar::get)
        }
    }

    fn list(&self) -> Vec<String> {
        read_lock!(self.variables)
            .keys()
            .map(ToString::to_string)
            .collect()
    }

    fn get_all(&self) -> HashMap<Box<str>, String> {
        read_lock!(self.variables)
            .iter()
            .filter_map(|(k, v)| v.get().map(|value| (k.clone(), value)))
            .collect()
    }

    fn namespaces(&self) -> Vec<String> {
        read_lock!(self.namespaces)
            .keys()
            .map(ToString::to_string)
            .collect()
    }

    fn get_namespace(&self, key: &str) -> Option<NamespaceTrait> {
        read_lock!(self.namespaces).get(key).cloned()
    }
}

impl WritableNamespace for VariableManager {
    /// Sets the value for a variable,
    /// creating it if it does not exist.
    fn set(&self, key: &str, value: String) -> Result<()> {
        if Self::key_is_valid(key) {
            if let Some(var) = write_lock!(self.variables).get_mut(&Box::from(key)) {
                var.set(Some(value));
            } else {
                let var = IronVar::new(Some(value));
                write_lock!(self.variables).insert(key.into(), var);
            }

            Ok(())
        } else {
            Err(Report::msg("Invalid key"))
        }
    }
}

/// Ironbar dynamic variable representation.
/// Interact with them through the `VARIABLE_MANAGER` `VariableManager` singleton.
#[derive(Debug)]
pub struct IronVar {
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
    pub fn get(&self) -> Option<String> {
        self.value.clone()
    }

    /// Sets the current variable value.
    /// The change is broadcast to all receivers.
    fn set(&mut self, value: Option<String>) {
        self.value.clone_from(&value);
        self.tx.send_expect(value);
    }

    /// Subscribes to the variable.
    /// The latest value is immediately sent to all receivers.
    fn subscribe(&self) -> broadcast::Receiver<Option<String>> {
        let rx = self.tx.subscribe();
        self.tx.send_expect(self.value.clone());
        rx
    }
}
