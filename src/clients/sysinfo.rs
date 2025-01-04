use crate::modules::sysinfo::Interval;
use crate::{lock, register_client};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Mutex;
use sysinfo::{Components, Disks, LoadAvg, Networks, RefreshKind, System};

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

#[derive(Debug)]
pub struct ValueSet {
    values: HashMap<Box<str>, Value>,
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
pub struct Value {
    value: f64,
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

#[derive(Debug)]
pub struct Client {
    system: Mutex<System>,
    disks: Mutex<Disks>,
    components: Mutex<Components>,
    networks: Mutex<Networks>,
    load_average: Mutex<LoadAvg>,
}

impl Client {
    pub fn new() -> Self {
        let refresh_kind = RefreshKind::everything().without_processes();

        let system = System::new_with_specifics(refresh_kind);
        let disks = Disks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let load_average = System::load_average();

        Self {
            system: Mutex::new(system),
            disks: Mutex::new(disks),
            components: Mutex::new(components),
            networks: Mutex::new(networks),
            load_average: Mutex::new(load_average),
        }
    }

    pub fn refresh_cpu(&self) {
        lock!(self.system).refresh_cpu_all();
    }

    pub fn refresh_memory(&self) {
        lock!(self.system).refresh_memory();
    }

    pub fn refresh_network(&self) {
        lock!(self.networks).refresh(true);
    }

    pub fn refresh_temps(&self) {
        lock!(self.components).refresh(true);
    }

    pub fn refresh_disks(&self) {
        lock!(self.disks).refresh(true);
    }

    pub fn refresh_load_average(&self) {
        *lock!(self.load_average) = System::load_average();
    }

    pub fn cpu_frequency(&self) -> ValueSet {
        lock!(self.system)
            .cpus()
            .iter()
            .map(|cpu| {
                (
                    cpu.name().into(),
                    Value::new_with_prefix(cpu.frequency() as f64, Prefix::Mega),
                )
            })
            .collect()
    }

    pub fn cpu_percent(&self) -> ValueSet {
        lock!(self.system)
            .cpus()
            .iter()
            .map(|cpu| (cpu.name().into(), Value::new(cpu.cpu_usage() as f64)))
            .collect()
    }

    pub fn memory_free(&self) -> Value {
        Value::new(lock!(self.system).free_memory() as f64)
    }

    pub fn memory_available(&self) -> Value {
        Value::new(lock!(self.system).available_memory() as f64)
    }

    pub fn memory_total(&self) -> Value {
        Value::new(lock!(self.system).total_memory() as f64)
    }

    pub fn memory_used(&self) -> Value {
        Value::new(lock!(self.system).used_memory() as f64)
    }

    pub fn memory_percent(&self) -> Value {
        let total = lock!(self.system).total_memory() as f64;
        let used = lock!(self.system).used_memory() as f64;

        Value::new(used / total * 100.0)
    }

    pub fn swap_free(&self) -> Value {
        Value::new(lock!(self.system).free_swap() as f64)
    }

    pub fn swap_total(&self) -> Value {
        Value::new(lock!(self.system).total_swap() as f64)
    }

    pub fn swap_used(&self) -> Value {
        Value::new(lock!(self.system).used_swap() as f64)
    }
    pub fn swap_percent(&self) -> Value {
        let total = lock!(self.system).total_swap() as f64;
        let used = lock!(self.system).used_swap() as f64;

        Value::new(used / total * 100.0)
    }

    pub fn temp_c(&self) -> ValueSet {
        lock!(self.components)
            .iter()
            .map(|comp| {
                (
                    comp.label().into(),
                    Value::new(comp.temperature().unwrap_or_default() as f64),
                )
            })
            .collect()
    }

    pub fn temp_f(&self) -> ValueSet {
        lock!(self.components)
            .iter()
            .map(|comp| {
                (
                    comp.label().into(),
                    Value::new(c_to_f(comp.temperature().unwrap_or_default() as f64)),
                )
            })
            .collect()
    }

    pub fn disk_free(&self) -> ValueSet {
        lock!(self.disks)
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(disk.available_space() as f64),
                )
            })
            .collect()
    }

    pub fn disk_total(&self) -> ValueSet {
        lock!(self.disks)
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(disk.total_space() as f64),
                )
            })
            .collect()
    }

    pub fn disk_used(&self) -> ValueSet {
        lock!(self.disks)
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new((disk.total_space() - disk.available_space()) as f64),
                )
            })
            .collect()
    }

    pub fn disk_percent(&self) -> ValueSet {
        lock!(self.disks)
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(
                        (disk.total_space() - disk.available_space()) as f64
                            / disk.total_space() as f64
                            * 100.0,
                    ),
                )
            })
            .collect()
    }

    pub fn disk_read(&self, interval: Interval) -> ValueSet {
        lock!(self.disks)
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(disk.usage().read_bytes as f64 / interval.disks() as f64),
                )
            })
            .collect()
    }

    pub fn disk_write(&self, interval: Interval) -> ValueSet {
        lock!(self.disks)
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(disk.usage().written_bytes as f64 / interval.disks() as f64),
                )
            })
            .collect()
    }

    pub fn net_down(&self, interval: Interval) -> ValueSet {
        lock!(self.networks)
            .iter()
            .map(|(name, net)| {
                (
                    name.as_str().into(),
                    Value::new(net.received() as f64 / interval.networks() as f64),
                )
            })
            .collect()
    }

    pub fn net_up(&self, interval: Interval) -> ValueSet {
        lock!(self.networks)
            .iter()
            .map(|(name, net)| {
                (
                    name.as_str().into(),
                    Value::new(net.transmitted() as f64 / interval.networks() as f64),
                )
            })
            .collect()
    }

    pub fn load_average_1(&self) -> Value {
        Value::new(lock!(self.load_average).one)
    }

    pub fn load_average_5(&self) -> Value {
        Value::new(lock!(self.load_average).five)
    }

    pub fn load_average_15(&self) -> Value {
        Value::new(lock!(self.load_average).fifteen)
    }

    /// Gets system uptime formatted as `HH:mm`.
    pub fn uptime(&self) -> String {
        let uptime = System::uptime();
        let hours = uptime / 3600;
        format!("{:0>2}:{:0>2}", hours, (uptime % 3600) / 60)
    }
}

register_client!(Client, sys_info);

const fn c_to_f(c: f64) -> f64 {
    c / 5.0 * 9.0 + 32.0
}
