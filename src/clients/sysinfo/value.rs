use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::time::{Duration, Instant};

#[repr(u64)]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum Prefix {
    #[default]
    None = 1,

    Kilo = 1000,
    Mega = Prefix::Kilo as u64 * 1000,
    Giga = Prefix::Mega as u64 * 1000,
    Tera = Prefix::Giga as u64 * 1000,
    Peta = Prefix::Tera as u64 * 1000,

    Kibi = 1024,
    Mebi = Prefix::Kibi as u64 * 1024,
    Gibi = Prefix::Mebi as u64 * 1024,
    Tebi = Prefix::Gibi as u64 * 1024,
    Pebi = Prefix::Tebi as u64 * 1024,

    // # Units
    // These are special cases
    // where you'd actually want to do slightly more than a prefix alone.
    // Included as part of the prefix system for simplicity.
    KiloBit = 128,
    MegaBit = Prefix::KiloBit as u64 * 1024,
    GigaBit = Prefix::MegaBit as u64 * 1024,
}

#[derive(Debug, Clone)]
pub enum Function {
    None,
    Sum,
    Min,
    Max,
    Mean,
    Name(String),
}

impl FromStr for Function {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sum" => Ok(Self::Sum),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "mean" => Ok(Self::Mean),
            "" => Err(()),
            _ => Ok(Self::Name(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "lua", derive(mlua::UserData))]
pub struct ValueSet {
    #[cfg_attr(feature = "lua", lua(skip))]
    pub(super) values: HashMap<Box<str>, Value>,
}

impl FromIterator<(Box<str>, Value)> for ValueSet {
    fn from_iter<T: IntoIterator<Item = (Box<str>, Value)>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

impl ValueSet {
    fn values(&self, prefix: Prefix) -> impl Iterator<Item = f64> + use<'_> {
        self.values
            .values()
            .map(move |v| v.get(prefix))
            .filter(|v| !v.is_nan())
    }

    pub fn apply(&self, function: &Function, prefix: Prefix) -> f64 {
        match function {
            Function::None => 0.0,
            Function::Sum => self.sum(prefix),
            Function::Min => self.min(prefix),
            Function::Max => self.max(prefix),
            Function::Mean => self.mean(prefix),
            Function::Name(name) => self
                .values
                .get(&Box::from(name.as_str()))
                .map(|v| v.get(prefix))
                .unwrap_or_default(),
        }
    }

    fn sum(&self, prefix: Prefix) -> f64 {
        self.values(prefix).sum()
    }

    fn min(&self, prefix: Prefix) -> f64 {
        self.values(prefix)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or_default()
    }

    fn max(&self, prefix: Prefix) -> f64 {
        self.values(prefix)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or_default()
    }

    fn mean(&self, prefix: Prefix) -> f64 {
        self.sum(prefix) / self.values.len() as f64
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "lua", derive(mlua::UserData))]
pub struct Value {
    #[cfg_attr(feature = "lua", lua(skip))]
    value: f64,
    #[cfg_attr(feature = "lua", lua(skip))]
    prefix: Prefix,
}

impl Value {
    pub fn new(value: f64) -> Self {
        Self::new_with_prefix(value, Prefix::None)
    }

    pub fn new_with_prefix(value: f64, prefix: Prefix) -> Self {
        Self { value, prefix }
    }

    pub fn get(self, prefix: Prefix) -> f64 {
        if prefix == self.prefix {
            self.value
        } else {
            let scale = self.prefix as u64 as f64 / prefix as u64 as f64;
            self.value * scale
        }
    }
}

/// Data whose values are deltas since the previous refresh,
/// such as disk and network throughput.
///
/// Refreshes don't happen at fixed intervals,
/// so rates are calculated from the time the deltas actually cover.
#[derive(Debug)]
pub(super) struct Delta<T> {
    pub data: T,
    pub refreshed_at: Instant,
    pub window: Duration,
}

impl<T> Delta<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            refreshed_at: Instant::now(),
            window: Duration::ZERO,
        }
    }

    pub fn refresh_with(&mut self, refresh: impl FnOnce(&mut T)) {
        let now = Instant::now();
        refresh(&mut self.data);
        self.window = now.duration_since(self.refreshed_at);
        self.refreshed_at = now;
    }

    /// Converts a delta into a per-second rate.
    pub fn rate(&self, delta: u64) -> f64 {
        if self.window.is_zero() {
            0.0
        } else {
            delta as f64 / self.window.as_secs_f64()
        }
    }
}

impl<T> Deref for Delta<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}
