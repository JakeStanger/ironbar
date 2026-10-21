use crate::{lock, register_client, spawn};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, LoadAvg, Networks, RefreshKind, System};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time;

/// Refreshes of the same kind closer together than this are skipped.
/// The cached values are still fresh, and shorter windows make rates noisy.
const MIN_REFRESH_GAP: Duration = Duration::from_millis(500);

/// A kind of data that is refreshed together.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RefreshType {
    Memory,
    Cpu,
    Temps,
    Disks,
    Network,
    System,
}

impl RefreshType {
    pub const ALL: [Self; 6] = [
        Self::Memory,
        Self::Cpu,
        Self::Temps,
        Self::Disks,
        Self::Network,
        Self::System,
    ];
}

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

/// Data whose values are deltas since the previous refresh,
/// such as disk and network throughput.
///
/// Refreshes don't happen at fixed intervals,
/// so rates are calculated from the time the deltas actually cover.
#[derive(Debug)]
struct Delta<T> {
    data: T,
    refreshed_at: Instant,
    window: Duration,
}

impl<T> Delta<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            refreshed_at: Instant::now(),
            window: Duration::ZERO,
        }
    }

    fn refresh_with(&mut self, refresh: impl FnOnce(&mut T)) {
        let now = Instant::now();
        refresh(&mut self.data);
        self.window = now.duration_since(self.refreshed_at);
        self.refreshed_at = now;
    }

    /// Converts a delta into a per-second rate.
    fn rate(&self, delta: u64) -> f64 {
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

/// A module waiting for refreshes of some kinds of data, each at its own interval.
#[derive(Debug)]
struct Subscriber {
    intervals: Vec<(RefreshType, u64)>,
    tx: mpsc::Sender<Vec<RefreshType>>,
}

impl Subscriber {
    /// Returns the kinds of data due at the given second.
    fn due(&self, tick: u64) -> impl Iterator<Item = RefreshType> + '_ {
        self.intervals
            .iter()
            .filter(move |&&(_, interval)| tick.is_multiple_of(interval))
            .map(|&(kind, _)| kind)
    }
}

#[derive(Debug, Default)]
struct Scheduler {
    subscribers: Vec<Subscriber>,
    running: bool,
}

#[derive(Debug)]
pub struct Client {
    system: Mutex<System>,
    disks: Mutex<Delta<Disks>>,
    components: Mutex<Components>,
    networks: Mutex<Delta<Networks>>,
    load_average: Mutex<LoadAvg>,
    refreshed_at: Mutex<HashMap<RefreshType, Instant>>,
    scheduler: Mutex<Scheduler>,
}

impl Client {
    pub fn new() -> Self {
        let refresh_kind = RefreshKind::everything().without_processes();

        let system = System::new_with_specifics(refresh_kind);
        let disks = Disks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let load_average = System::load_average();

        let now = Instant::now();

        Self {
            system: Mutex::new(system),
            disks: Mutex::new(Delta::new(disks)),
            components: Mutex::new(components),
            networks: Mutex::new(Delta::new(networks)),
            load_average: Mutex::new(load_average),
            refreshed_at: Mutex::new(RefreshType::ALL.map(|kind| (kind, now)).into()),
            scheduler: Mutex::new(Scheduler::default()),
        }
    }

    /// Subscribes to refreshes of the given kinds of data,
    /// each at its interval in seconds.
    ///
    /// Every kind is refreshed once per due second, no matter how many modules subscribe to it.
    /// The receiver gets the kinds that were refreshed, and closes immediately if none are given.
    pub fn subscribe(
        self: &Arc<Self>,
        intervals: Vec<(RefreshType, u64)>,
    ) -> mpsc::Receiver<Vec<RefreshType>> {
        let (tx, rx) = mpsc::channel(4);

        if intervals.is_empty() {
            return rx;
        }

        for &(kind, _) in &intervals {
            self.refresh(kind);
        }

        let intervals = intervals
            .into_iter()
            .map(|(kind, interval)| (kind, interval.max(1)))
            .collect();

        let mut scheduler = lock!(self.scheduler);
        scheduler.subscribers.push(Subscriber { intervals, tx });

        if !scheduler.running {
            scheduler.running = true;
            spawn(self.clone().run_scheduler());
        }

        rx
    }

    /// Sleeps until the next second any subscriber is due,
    /// refreshes the due kinds of data once and notifies the due subscribers.
    /// Stops once all subscribers are gone.
    async fn run_scheduler(self: Arc<Self>) {
        let start = time::Instant::now();
        let mut tick = 0;

        loop {
            let next_tick = {
                let mut scheduler = lock!(self.scheduler);
                scheduler
                    .subscribers
                    .retain(|subscriber| !subscriber.tx.is_closed());

                let next_tick = scheduler
                    .subscribers
                    .iter()
                    .flat_map(|subscriber| &subscriber.intervals)
                    .map(|&(_, interval)| (tick / interval + 1) * interval)
                    .min();

                let Some(next_tick) = next_tick else {
                    scheduler.running = false;
                    return;
                };

                next_tick
            };

            time::sleep_until(start + Duration::from_secs(next_tick)).await;
            tick = next_tick;

            let mut due = Vec::new();
            for kind in lock!(self.scheduler)
                .subscribers
                .iter()
                .flat_map(|subscriber| subscriber.due(tick))
            {
                if !due.contains(&kind) {
                    due.push(kind);
                }
            }

            for kind in due {
                self.refresh(kind);
            }

            for subscriber in &lock!(self.scheduler).subscribers {
                let refreshed: Vec<_> = subscriber.due(tick).collect();
                if !refreshed.is_empty() {
                    // if the channel is full, the module is still busy rendering
                    // and will pick up the refreshed data with its next update anyway
                    let _ = subscriber.tx.try_send(refreshed);
                }
            }
        }
    }

    /// Refreshes one kind of data,
    /// unless it was refreshed less than [`MIN_REFRESH_GAP`] ago.
    fn refresh(&self, kind: RefreshType) {
        {
            let now = Instant::now();
            let mut refreshed_at = lock!(self.refreshed_at);

            if refreshed_at
                .get(&kind)
                .is_some_and(|&at| now.duration_since(at) < MIN_REFRESH_GAP)
            {
                return;
            }

            refreshed_at.insert(kind, now);
        }

        match kind {
            RefreshType::Memory => lock!(self.system).refresh_memory(),
            RefreshType::Cpu => lock!(self.system).refresh_cpu_all(),
            RefreshType::Temps => lock!(self.components).refresh(true),
            RefreshType::Disks => lock!(self.disks).refresh_with(|disks| disks.refresh(true)),
            RefreshType::Network => {
                lock!(self.networks).refresh_with(|networks| networks.refresh(true));
            }
            RefreshType::System => *lock!(self.load_average) = System::load_average(),
        }
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

    pub fn disk_read(&self) -> ValueSet {
        let disks = lock!(self.disks);
        disks
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(disks.rate(disk.usage().read_bytes)),
                )
            })
            .collect()
    }

    pub fn disk_write(&self) -> ValueSet {
        let disks = lock!(self.disks);
        disks
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(disks.rate(disk.usage().written_bytes)),
                )
            })
            .collect()
    }

    pub fn net_down(&self) -> ValueSet {
        let networks = lock!(self.networks);
        networks
            .iter()
            .map(|(name, net)| {
                (
                    name.as_str().into(),
                    Value::new(networks.rate(net.received())),
                )
            })
            .collect()
    }

    pub fn net_up(&self) -> ValueSet {
        let networks = lock!(self.networks);
        networks
            .iter()
            .map(|(name, net)| {
                (
                    name.as_str().into(),
                    Value::new(networks.rate(net.transmitted())),
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

#[derive(Debug, Error)]
#[error("Invalid token type: {input_token}")]
pub struct InvalidTokenError {
    pub input_token: String,
}

impl FromStr for TokenType {
    type Err = InvalidTokenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
            _ => Err(InvalidTokenError {
                input_token: s.to_string(),
            }),
        }
    }
}

impl TokenType {
    /// The kind of data this token's value comes from.
    pub const fn refresh_type(self) -> RefreshType {
        match self {
            Self::CpuFrequency | Self::CpuPercent => RefreshType::Cpu,
            Self::MemoryFree
            | Self::MemoryAvailable
            | Self::MemoryTotal
            | Self::MemoryUsed
            | Self::MemoryPercent
            | Self::SwapFree
            | Self::SwapTotal
            | Self::SwapUsed
            | Self::SwapPercent => RefreshType::Memory,
            Self::TempC | Self::TempF => RefreshType::Temps,
            Self::DiskFree
            | Self::DiskTotal
            | Self::DiskUsed
            | Self::DiskPercent
            | Self::DiskRead
            | Self::DiskWrite => RefreshType::Disks,
            Self::NetDown | Self::NetUp => RefreshType::Network,
            Self::LoadAverage1 | Self::LoadAverage5 | Self::LoadAverage15 | Self::Uptime => {
                RefreshType::System
            }
        }
    }
}

#[cfg(any(feature = "ipc", feature = "cairo"))]
use crate::ironvar::{Namespace, NamespaceTrait};

#[cfg(any(feature = "ipc", feature = "cairo"))]
impl Namespace for Client {
    fn get(&self, key: &str) -> Option<String> {
        let get = |value: Value| Some(value.get(Prefix::None).to_string());

        let token = TokenType::from_str(key).ok()?;
        // modules only refresh the data they display
        self.refresh(token.refresh_type());

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
        self.refresh(token.refresh_type());

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
            TokenType::DiskRead => Some(Arc::new(self.disk_read())),
            TokenType::DiskWrite => Some(Arc::new(self.disk_write())),
            TokenType::NetDown => Some(Arc::new(self.net_down())),
            TokenType::NetUp => Some(Arc::new(self.net_up())),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_rate_uses_elapsed_time() {
        let mut delta = Delta {
            data: 0,
            refreshed_at: Instant::now() - Duration::from_secs(2),
            window: Duration::ZERO,
        };
        assert!(delta.rate(1000).abs() < f64::EPSILON);

        delta.refresh_with(|data| *data += 1);
        assert_eq!(*delta, 1);
        assert!((delta.rate(1000) - 500.0).abs() < 1.0);
    }

    #[test]
    fn subscriber_is_due_at_multiples_of_its_intervals() {
        let subscriber = Subscriber {
            intervals: vec![(RefreshType::Cpu, 2), (RefreshType::Network, 3)],
            tx: mpsc::channel(1).0,
        };
        let due = |tick| subscriber.due(tick).collect::<Vec<_>>();

        assert!(due(1).is_empty());
        assert_eq!(due(2), [RefreshType::Cpu]);
        assert_eq!(due(3), [RefreshType::Network]);
        assert_eq!(due(6), [RefreshType::Cpu, RefreshType::Network]);
    }
}
