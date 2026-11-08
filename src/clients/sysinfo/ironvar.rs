use super::{Client, Function, Prefix, TokenType, Value, ValueSet};
use crate::ironvar::{Namespace, NamespaceTrait};
use std::str::FromStr;
use std::sync::Arc;

impl Namespace for Client {
    fn get(&self, key: &str) -> Option<String> {
        let get = |value: Value| Some(value.get(Prefix::None).to_string());

        let data = self.data().ok()?;
        let token = TokenType::from_str(key).ok()?;
        // modules only refresh the data they display
        data.refresh(token.refresh_type());

        match token {
            TokenType::CpuFrequency => None,
            TokenType::CpuPercent => None,
            TokenType::MemoryFree => get(data.memory_free()),
            TokenType::MemoryAvailable => get(data.memory_available()),
            TokenType::MemoryTotal => get(data.memory_total()),
            TokenType::MemoryUsed => get(data.memory_used()),
            TokenType::MemoryPercent => get(data.memory_percent()),
            TokenType::SwapFree => get(data.swap_free()),
            TokenType::SwapTotal => get(data.swap_total()),
            TokenType::SwapUsed => get(data.swap_used()),
            TokenType::SwapPercent => get(data.swap_percent()),
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
            TokenType::LoadAverage1 => get(data.load_average_1()),
            TokenType::LoadAverage5 => get(data.load_average_5()),
            TokenType::LoadAverage15 => get(data.load_average_15()),
            TokenType::Uptime => Client::uptime(),
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
        let data = self.data().ok()?;
        let token = TokenType::from_str(key).ok()?;
        data.refresh(token.refresh_type());

        match token {
            TokenType::CpuFrequency => Some(Arc::new(data.cpu_frequency())),
            TokenType::CpuPercent => Some(Arc::new(data.cpu_percent())),
            TokenType::MemoryFree => None,
            TokenType::MemoryAvailable => None,
            TokenType::MemoryTotal => None,
            TokenType::MemoryUsed => None,
            TokenType::MemoryPercent => None,
            TokenType::SwapFree => None,
            TokenType::SwapTotal => None,
            TokenType::SwapUsed => None,
            TokenType::SwapPercent => None,
            TokenType::TempC => Some(Arc::new(data.temp_c())),
            TokenType::TempF => Some(Arc::new(data.temp_f())),
            TokenType::DiskFree => Some(Arc::new(data.disk_free())),
            TokenType::DiskTotal => Some(Arc::new(data.disk_total())),
            TokenType::DiskUsed => Some(Arc::new(data.disk_used())),
            TokenType::DiskPercent => Some(Arc::new(data.disk_percent())),
            TokenType::DiskRead => Some(Arc::new(data.disk_read())),
            TokenType::DiskWrite => Some(Arc::new(data.disk_write())),
            TokenType::NetDown => Some(Arc::new(data.net_down())),
            TokenType::NetUp => Some(Arc::new(data.net_up())),
            TokenType::LoadAverage1 => None,
            TokenType::LoadAverage5 => None,
            TokenType::LoadAverage15 => None,
            TokenType::Uptime => None,
        }
    }
}

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
