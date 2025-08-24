use crate::modules::sysinfo::Interval;
use crate::{lock, register_client};
use color_eyre::{Report, Result};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
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
    pub fn uptime() -> String {
        let uptime = System::uptime();
        let hours = uptime / 3600;
        format!("{:0>2}:{:0>2}", hours, (uptime % 3600) / 60)
    }
}

register_client!(Client, sys_info);

const fn c_to_f(c: f64) -> f64 {
    c / 5.0 * 9.0 + 32.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    CpuFrequency,
    CpuPercent,

    MemoryFree,
    MemoryAvailable,
    MemoryTotal,
    MemoryUsed,
    MemoryPercent,

    SwapFree,
    SwapTotal,
    SwapUsed,
    SwapPercent,

    TempC,
    TempF,

    DiskFree,
    DiskTotal,
    DiskUsed,
    DiskPercent,
    DiskRead,
    DiskWrite,

    NetDown,
    NetUp,

    LoadAverage1,
    LoadAverage5,
    LoadAverage15,
    Uptime,
}

impl FromStr for TokenType {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "cpu_frequency" => Ok(Self::CpuFrequency),
            "cpu_percent" => Ok(Self::CpuPercent),

            "memory_free" => Ok(Self::MemoryFree),
            "memory_available" => Ok(Self::MemoryAvailable),
            "memory_total" => Ok(Self::MemoryTotal),
            "memory_used" => Ok(Self::MemoryUsed),
            "memory_percent" => Ok(Self::MemoryPercent),

            "swap_free" => Ok(Self::SwapFree),
            "swap_total" => Ok(Self::SwapTotal),
            "swap_used" => Ok(Self::SwapUsed),
            "swap_percent" => Ok(Self::SwapPercent),

            "temp_c" => Ok(Self::TempC),
            "temp_f" => Ok(Self::TempF),

            "disk_free" => Ok(Self::DiskFree),
            "disk_total" => Ok(Self::DiskTotal),
            "disk_used" => Ok(Self::DiskUsed),
            "disk_percent" => Ok(Self::DiskPercent),
            "disk_read" => Ok(Self::DiskRead),
            "disk_write" => Ok(Self::DiskWrite),

            "net_down" => Ok(Self::NetDown),
            "net_up" => Ok(Self::NetUp),

            "load_average_1" => Ok(Self::LoadAverage1),
            "load_average_5" => Ok(Self::LoadAverage5),
            "load_average_15" => Ok(Self::LoadAverage15),
            "uptime" => Ok(Self::Uptime),
            _ => Err(Report::msg(format!("invalid token type: '{s}'"))),
        }
    }
}

#[cfg(feature = "ipc")]
use crate::ironvar::Namespace;
use crate::ironvar::NamespaceTrait;

#[cfg(feature = "ipc")]
impl Namespace for Client {
    fn get(&self, key: &str) -> Option<String> {
        let get = |value: Value| Some(value.get(Prefix::None).to_string());

        let token = TokenType::from_str(key).ok()?;
        match token {
            TokenType::CpuFrequency => None,
            TokenType::CpuPercent => None,
            TokenType::MemoryFree => get(self.memory_free()),
            TokenType::MemoryAvailable => get(self.memory_available()),
            TokenType::MemoryTotal => get(self.memory_total()),
            TokenType::MemoryUsed => get(self.memory_used()),
            TokenType::MemoryPercent => get(self.memory_percent()),
            TokenType::SwapFree => get(self.swap_free()),
            TokenType::SwapTotal => get(self.swap_total()),
            TokenType::SwapUsed => get(self.swap_used()),
            TokenType::SwapPercent => get(self.swap_percent()),
            TokenType::TempC => None,
            TokenType::TempF => None,
            TokenType::DiskFree => None,
            TokenType::DiskTotal => None,
            TokenType::DiskUsed => None,
            TokenType::DiskPercent => None,
            TokenType::DiskRead => None,
            TokenType::DiskWrite => None,
            TokenType::NetDown => None,
            TokenType::NetUp => None,
            TokenType::LoadAverage1 => get(self.load_average_1()),
            TokenType::LoadAverage5 => get(self.load_average_5()),
            TokenType::LoadAverage15 => get(self.load_average_15()),
            TokenType::Uptime => Some(Client::uptime()),
        }
    }

    fn list(&self) -> Vec<String> {
        vec![
            "memory_free",
            "memory_available",
            "memory_total",
            "memory_used",
            "memory_percent",
            "swap_free",
            "swap_total",
            "swap_used",
            "swap_percent",
            "load_average_1",
            "load_average_5",
            "load_average_15",
            "uptime",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect()
    }

    fn namespaces(&self) -> Vec<String> {
        vec![
            "cpu_frequency",
            "cpu_percent",
            "temp_c",
            "temp_f",
            "disk_free",
            "disk_total",
            "disk_used",
            "disk_percent",
            "disk_read",
            "disk_write",
            "net_down",
            "net_up",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect()
    }

    fn get_namespace(&self, key: &str) -> Option<NamespaceTrait> {
        let token = TokenType::from_str(key).ok()?;

        match token {
            TokenType::CpuFrequency => Some(Arc::new(self.cpu_frequency())),
            TokenType::CpuPercent => Some(Arc::new(self.cpu_percent())),
            TokenType::MemoryFree => None,
            TokenType::MemoryAvailable => None,
            TokenType::MemoryTotal => None,
            TokenType::MemoryUsed => None,
            TokenType::MemoryPercent => None,
            TokenType::SwapFree => None,
            TokenType::SwapTotal => None,
            TokenType::SwapUsed => None,
            TokenType::SwapPercent => None,
            TokenType::TempC => Some(Arc::new(self.temp_c())),
            TokenType::TempF => Some(Arc::new(self.temp_f())),
            TokenType::DiskFree => Some(Arc::new(self.disk_free())),
            TokenType::DiskTotal => Some(Arc::new(self.disk_total())),
            TokenType::DiskUsed => Some(Arc::new(self.disk_used())),
            TokenType::DiskPercent => Some(Arc::new(self.disk_percent())),
            TokenType::DiskRead => Some(Arc::new(self.disk_read(Interval::All(1)))),
            TokenType::DiskWrite => Some(Arc::new(self.disk_write(Interval::All(1)))),
            TokenType::NetDown => Some(Arc::new(self.net_down(Interval::All(1)))),
            TokenType::NetUp => Some(Arc::new(self.net_up(Interval::All(1)))),
            TokenType::LoadAverage1 => None,
            TokenType::LoadAverage5 => None,
            TokenType::LoadAverage15 => None,
            TokenType::Uptime => None,
        }
    }
}

#[cfg(feature = "ipc")]
impl Namespace for ValueSet {
    fn get(&self, key: &str) -> Option<String> {
        let function = Function::from_str(key).ok()?;
        Some(self.apply(&function, Prefix::None).to_string())
    }

    fn list(&self) -> Vec<String> {
        let mut vec = vec!["sum", "min", "max", "mean"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        vec.extend(self.values.keys().map(ToString::to_string));
        vec
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _key: &str) -> Option<Arc<dyn Namespace + Sync + Send>> {
        None
    }
}
