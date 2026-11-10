#[cfg(any(feature = "ipc", feature = "lua"))]
pub mod ironvar;
#[cfg(feature = "lua")]
pub mod lua;
pub mod value;

use crate::channels::BroadcastChannel;
#[cfg(feature = "lua")]
use crate::clients::lua::LuaClient;
use crate::{lock, register_fallible_client, spawn};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{
    Component, Components, Cpu, Disk, Disks, Gpus, LoadAvg, NetworkData, Networks, RefreshKind,
    System,
};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::warn;
use value::Delta;

pub use value::{Function, Prefix, Value, ValueSet};

/// Refreshes of the same kind closer together than this are skipped.
/// The cached values are still fresh, and shorter windows make rates noisy.
const MIN_REFRESH_GAP: Duration = Duration::from_millis(500);

/// A kind of data that is refreshed together.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "lua", derive(mlua::UserData))]
pub enum RefreshType {
    Memory,
    Cpu,
    Gpu,
    Temps,
    Disks,
    Network,
    System,
}

impl RefreshType {
    pub const ALL: [Self; 7] = [
        Self::Memory,
        Self::Cpu,
        Self::Gpu,
        Self::Temps,
        Self::Disks,
        Self::Network,
        Self::System,
    ];
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

    GpuPercent,
    GpuMemoryFree,
    GpuMemoryTotal,
    GpuMemoryUsed,
    GpuMemoryPercent,

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

            "gpu_percent" => Ok(Self::GpuPercent),
            "gpu_memory_free" => Ok(Self::GpuMemoryFree),
            "gpu_memory_total" => Ok(Self::GpuMemoryTotal),
            "gpu_memory_used" => Ok(Self::GpuMemoryUsed),
            "gpu_memory_percent" => Ok(Self::GpuMemoryPercent),

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
            Self::GpuPercent
            | Self::GpuMemoryFree
            | Self::GpuMemoryTotal
            | Self::GpuMemoryUsed
            | Self::GpuMemoryPercent => RefreshType::Gpu,
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
    /// Handle to the scheduler task, if one is running.
    handle: Option<JoinHandle<()>>,
}

/// Full snapshot of all system information values.
#[allow(dead_code)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "lua", derive(mlua::UserData))]
pub struct State {
    #[cfg_attr(feature = "lua", lua(get))]
    pub cpu_frequency: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub cpu_percent: ValueSet,

    #[cfg_attr(feature = "lua", lua(get))]
    pub memory_free: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub memory_available: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub memory_total: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub memory_used: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub memory_percent: Value,

    #[cfg_attr(feature = "lua", lua(get))]
    pub gpu_percent: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub gpu_memory_free: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub gpu_memory_total: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub gpu_memory_used: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub gpu_memory_percent: ValueSet,

    #[cfg_attr(feature = "lua", lua(get))]
    pub swap_free: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub swap_total: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub swap_used: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub swap_percent: Value,

    #[cfg_attr(feature = "lua", lua(get))]
    pub temp_c: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub temp_f: ValueSet,

    #[cfg_attr(feature = "lua", lua(get))]
    pub disk_free: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub disk_total: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub disk_used: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub disk_percent: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub disk_read: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub disk_write: ValueSet,

    #[cfg_attr(feature = "lua", lua(get))]
    pub net_down: ValueSet,
    #[cfg_attr(feature = "lua", lua(get))]
    pub net_up: ValueSet,

    #[cfg_attr(feature = "lua", lua(get))]
    pub load_average_1: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub load_average_5: Value,
    #[cfg_attr(feature = "lua", lua(get))]
    pub load_average_15: Value,

    #[cfg_attr(feature = "lua", lua(get))]
    pub uptime: Option<String>,
}

#[derive(Debug)]
pub enum Client {
    Stopped(BroadcastChannel<RefreshType>),
    Started {
        channel: BroadcastChannel<RefreshType>,
        data: Arc<Data>,
    },
}

impl Client {
    pub fn new() -> Self {
        Self::Stopped(BroadcastChannel::new(16))
    }

    /// Gets the underlying system data.
    pub fn data(&self) -> super::Result<&Arc<Data>, sysinfo::Error> {
        match self {
            Self::Stopped(_) => Err(super::Error::InvalidState),
            Self::Started { data, .. } => Ok(data),
        }
    }

    /// Subscribes to refreshes of the given kinds of data,
    /// each at its interval in seconds.
    ///
    /// See [`Data::subscribe_intervals`].
    pub fn subscribe_intervals(
        &self,
        intervals: Vec<(RefreshType, u64)>,
    ) -> super::Result<mpsc::Receiver<Vec<RefreshType>>, sysinfo::Error> {
        Ok(self.data()?.subscribe_intervals(intervals))
    }

    /// Gets system uptime formatted as `HH:mm`.
    pub fn uptime() -> Option<String> {
        let uptime = System::uptime().ok()?;
        let hours = uptime / 3600;
        Some(format!("{:0>2}:{:0>2}", hours, (uptime % 3600) / 60))
    }
}

impl super::Client for Client {
    type State = State;
    type Event = RefreshType;
    type Error = sysinfo::Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> Result<Self, Self::Error> {
        let Self::Stopped(channel) = self else {
            return Ok(self);
        };

        let data = Data::new(channel.clone())?;

        Ok(Self::Started {
            channel,
            data: Arc::new(data),
        })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        let Self::Started { channel, data } = self else {
            return Ok(self);
        };

        if let Some(handle) = lock!(data.scheduler).handle.take() {
            handle.abort();
        }

        Ok(Self::Stopped(channel))
    }

    async fn state(&self) -> super::Result<Self::State, Self::Error> {
        let data = self.data()?;

        for kind in RefreshType::ALL {
            data.refresh(kind);
        }

        Ok(State {
            cpu_frequency: data.cpu_frequency(),
            cpu_percent: data.cpu_percent(),
            memory_free: data.memory_free(),
            memory_available: data.memory_available(),
            memory_total: data.memory_total(),
            memory_used: data.memory_used(),
            memory_percent: data.memory_percent(),
            swap_free: data.swap_free(),
            swap_total: data.swap_total(),
            swap_used: data.swap_used(),
            swap_percent: data.swap_percent(),
            gpu_percent: data.gpu_percent(),
            gpu_memory_free: data.gpu_memory_free(),
            gpu_memory_total: data.gpu_memory_total(),
            gpu_memory_used: data.gpu_memory_used(),
            gpu_memory_percent: data.gpu_memory_percent(),
            temp_c: data.temp_c(),
            temp_f: data.temp_f(),
            disk_free: data.disk_free(),
            disk_total: data.disk_total(),
            disk_used: data.disk_used(),
            disk_percent: data.disk_percent(),
            disk_read: data.disk_read(),
            disk_write: data.disk_write(),
            net_down: data.net_down(),
            net_up: data.net_up(),
            load_average_1: data.load_average_1(),
            load_average_5: data.load_average_5(),
            load_average_15: data.load_average_15(),
            uptime: Self::uptime(),
        })
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        match self {
            Self::Stopped(channel) | Self::Started { channel, .. } => channel.subscribe(),
        }
    }

    #[cfg(feature = "lua")]
    fn add_lua_methods<M: mlua::UserDataMethods<LuaClient<Self>>>(methods: &mut M) {
        lua::add_methods(methods);
    }
}

/// Live system data
#[derive(Debug)]
pub struct Data {
    system: Mutex<System>,
    gpu_worker: GpuWorker,
    gpus: Mutex<Vec<GpuStats>>,
    disks: Mutex<Delta<Disks>>,
    components: Mutex<Components>,
    networks: Mutex<Delta<Networks>>,
    load_average: Mutex<LoadAvg>,
    refreshed_at: Mutex<HashMap<RefreshType, Instant>>,
    scheduler: Mutex<Scheduler>,
    tx: broadcast::Sender<RefreshType>,
}

impl Data {
    fn new(tx: broadcast::Sender<RefreshType>) -> Result<Self, sysinfo::Error> {
        let refresh_kind = RefreshKind::everything().without_processes();

        let system = System::new_with_specifics(refresh_kind)?;
        let (gpu_worker, gpus) = GpuWorker::spawn()?;
        let disks = Disks::new_with_refreshed_list()?;
        let components = Components::new_with_refreshed_list()?;
        let networks = Networks::new_with_refreshed_list()?;
        let load_average = System::load_average()?;

        let now = Instant::now();

        Ok(Self {
            system: Mutex::new(system),
            gpu_worker,
            gpus: Mutex::new(gpus),
            disks: Mutex::new(Delta::new(disks)),
            components: Mutex::new(components),
            networks: Mutex::new(Delta::new(networks)),
            load_average: Mutex::new(load_average),
            refreshed_at: Mutex::new(RefreshType::ALL.map(|kind| (kind, now)).into()),
            scheduler: Mutex::new(Scheduler::default()),
            tx,
        })
    }

    /// Subscribes to refreshes of the given kinds of data,
    /// each at its interval in seconds.
    ///
    /// Every kind is refreshed once per due second, no matter how many modules subscribe to it.
    /// The receiver gets the kinds that were refreshed, and closes immediately if none are given.
    pub fn subscribe_intervals(
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

        if scheduler.handle.is_none() {
            scheduler.handle = Some(spawn(self.clone().run_scheduler()));
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
                    scheduler.handle = None;
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
            RefreshType::Gpu => match self.gpu_worker.refresh() {
                Some(gpus) => *lock!(self.gpus) = gpus,
                None => warn!("failed to refresh gpus: worker thread is not running"),
            },
            RefreshType::Temps => lock!(self.components).refresh(true),
            RefreshType::Disks => lock!(self.disks).refresh_with(|disks| disks.refresh(true)),
            RefreshType::Network => {
                lock!(self.networks).refresh_with(|networks| networks.refresh(true));
            }
            RefreshType::System => match System::load_average() {
                Ok(load_average) => *lock!(self.load_average) = load_average,
                Err(err) => warn!("failed to refresh load average: {err}"),
            },
        }

        // ignore error if nobody listening
        let _ = self.tx.send(kind);
    }

    pub fn cpu_frequency(&self) -> ValueSet {
        self.map_cpus(|cpu| Value::new_with_prefix(cpu.frequency() as f64, Prefix::Mega))
    }

    pub fn cpu_percent(&self) -> ValueSet {
        self.map_cpus(|cpu| Value::new(cpu.usage() as f64))
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

    pub fn gpu_percent(&self) -> ValueSet {
        self.map_gpus(|gpu| Value::new(gpu.usage.unwrap_or_default() as f64))
    }

    pub fn gpu_memory_free(&self) -> ValueSet {
        self.map_gpus(|gpu| {
            Value::new(
                gpu.total_memory
                    .unwrap_or_default()
                    .saturating_sub(gpu.used_memory.unwrap_or_default()) as f64,
            )
        })
    }

    pub fn gpu_memory_total(&self) -> ValueSet {
        self.map_gpus(|gpu| Value::new(gpu.total_memory.unwrap_or_default() as f64))
    }

    pub fn gpu_memory_used(&self) -> ValueSet {
        self.map_gpus(|gpu| Value::new(gpu.used_memory.unwrap_or_default() as f64))
    }

    pub fn gpu_memory_percent(&self) -> ValueSet {
        self.map_gpus(|gpu| {
            let total = gpu.total_memory.unwrap_or_default() as f64;
            let used = gpu.used_memory.unwrap_or_default() as f64;

            Value::new(if total > 0.0 {
                used / total * 100.0
            } else {
                0.0
            })
        })
    }

    pub fn temp_c(&self) -> ValueSet {
        self.map_components(|comp| comp.temperature().unwrap_or_default() as f64)
    }

    pub fn temp_f(&self) -> ValueSet {
        const fn c_to_f(c: f64) -> f64 {
            c / 5.0 * 9.0 + 32.0
        }

        self.map_components(|comp| c_to_f(comp.temperature().unwrap_or_default() as f64))
    }

    pub fn disk_free(&self) -> ValueSet {
        self.map_disks(|disk, _| disk.available_space() as f64)
    }

    pub fn disk_total(&self) -> ValueSet {
        self.map_disks(|disk, _| disk.total_space() as f64)
    }

    pub fn disk_used(&self) -> ValueSet {
        self.map_disks(|disk, _| (disk.total_space() - disk.available_space()) as f64)
    }

    pub fn disk_percent(&self) -> ValueSet {
        self.map_disks(|disk, _| {
            (disk.total_space() - disk.available_space()) as f64 / disk.total_space() as f64 * 100.0
        })
    }

    pub fn disk_read(&self) -> ValueSet {
        self.map_disks(|disk, disks| disks.rate(disk.usage().read_bytes))
    }

    pub fn disk_write(&self) -> ValueSet {
        self.map_disks(|disk, disks| disks.rate(disk.usage().written_bytes))
    }

    pub fn net_down(&self) -> ValueSet {
        self.map_networks(|net, networks| networks.rate(net.received()))
    }

    pub fn net_up(&self) -> ValueSet {
        self.map_networks(|net, networks| networks.rate(net.transmitted()))
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

    #[inline]
    fn map_cpus(&self, value: impl Fn(&Cpu) -> Value) -> ValueSet {
        lock!(self.system)
            .cpus()
            .iter()
            .map(|cpu| (cpu.name().into(), value(cpu)))
            .collect()
    }

    #[inline]
    fn map_gpus(&self, value: impl Fn(&GpuStats) -> Value) -> ValueSet {
        lock!(self.gpus)
            .iter()
            .map(|gpu| (gpu.pci.as_str().into(), value(gpu)))
            .collect()
    }

    #[inline]
    fn map_components(&self, value: impl Fn(&Component) -> f64) -> ValueSet {
        lock!(self.components)
            .iter()
            .map(|component| (component.label().into(), Value::new(value(component))))
            .collect()
    }

    #[inline]
    fn map_disks(&self, value: impl Fn(&Disk, &Delta<Disks>) -> f64) -> ValueSet {
        let disks = lock!(self.disks);
        disks
            .iter()
            .map(|disk| {
                (
                    disk.mount_point().to_string_lossy().into(),
                    Value::new(value(disk, &disks)),
                )
            })
            .collect()
    }

    #[inline]
    fn map_networks(&self, value: impl Fn(&NetworkData, &Delta<Networks>) -> f64) -> ValueSet {
        let networks = lock!(self.networks);
        networks
            .iter()
            .map(|(name, network)| {
                (
                    name.to_string_lossy().into(),
                    Value::new(value(network, &networks)),
                )
            })
            .collect()
    }
}

/// A `Send` snapshot of a single GPU's stats.
///
/// [`Gpus`] holds raw handles to dynamically loaded libraries (NVML, Vulkan) so is `!Send`.
#[derive(Debug, Clone)]
struct GpuStats {
    pci: String,
    usage: Option<f32>,
    total_memory: Option<u64>,
    used_memory: Option<u64>,
}

impl GpuStats {
    fn snapshot(gpus: &Gpus) -> Vec<Self> {
        gpus.iter()
            .map(|gpu| Self {
                pci: gpu.pci().to_string(),
                usage: gpu.usage(),
                total_memory: gpu.total_memory(),
                used_memory: gpu.used_memory(),
            })
            .collect()
    }
}

/// Handle to a dedicated thread which owns the `!Send` [`Gpus`].
#[derive(Debug)]
struct GpuWorker {
    tx: std::sync::mpsc::Sender<std::sync::mpsc::SyncSender<Vec<GpuStats>>>,
}

impl GpuWorker {
    fn spawn() -> Result<(Self, Vec<GpuStats>), sysinfo::Error> {
        use std::sync::mpsc;

        let (init_tx, init_rx) = mpsc::sync_channel(1);
        let (tx, rx) = mpsc::channel::<mpsc::SyncSender<Vec<GpuStats>>>();

        let spawned = thread::Builder::new()
            .name("ironbar-sysinfo-gpu".to_string())
            .spawn(move || {
                let mut gpus = match Gpus::new_with_refreshed_list() {
                    Ok(gpus) => {
                        let _ = init_tx.send(Ok(GpuStats::snapshot(&gpus)));
                        gpus
                    }
                    Err(err) => {
                        let _ = init_tx.send(Err(err));
                        return;
                    }
                };

                for reply in rx {
                    gpus.refresh(true);
                    let _ = reply.send(GpuStats::snapshot(&gpus));
                }
            });

        if let Err(err) = spawned {
            warn!("failed to spawn gpu worker thread: {err}");
            return Ok((Self { tx }, Vec::new()));
        }

        match init_rx.recv() {
            Ok(Ok(gpus)) => Ok((Self { tx }, gpus)),
            Ok(Err(err)) => Err(err),
            Err(_) => {
                warn!("gpu worker thread exited during initialisation");
                Ok((Self { tx }, Vec::new()))
            }
        }
    }

    /// Refreshes the GPU data.
    ///
    /// Returns `None` if the worker thread is not running.
    fn refresh(&self) -> Option<Vec<GpuStats>> {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        self.tx.send(reply_tx).ok()?;
        reply_rx.recv().ok()
    }
}

register_fallible_client!(Client, sys_info);

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
