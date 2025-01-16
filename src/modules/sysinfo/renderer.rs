use super::token::{Alignment, Part, Token, TokenType};
use super::Interval;
use crate::clients;
use crate::clients::sysinfo::{Value, ValueSet};

impl Part {
    pub fn render_all(
        tokens: &[Self],
        client: &clients::sysinfo::Client,
        interval: Interval,
    ) -> String {
        tokens
            .iter()
            .map(|part| part.render(client, interval))
            .collect()
    }

    fn render(&self, client: &clients::sysinfo::Client, interval: Interval) -> String {
        match self {
            Part::Static(str) => str.clone(),
            Part::Token(token) => {
                let value = token.get(client, interval);

                let fmt = token.formatting;
                let mut str = format!("{value:.precision$}", precision = fmt.precision);

                // fill/align doesn't support parameterization so we need our own impl
                let mut add_to_end = fmt.align == Alignment::Right;
                while str.len() < fmt.width {
                    if add_to_end {
                        str.push(fmt.fill);
                    } else {
                        str.insert(0, fmt.fill);
                    }

                    if fmt.align == Alignment::Center {
                        add_to_end = !add_to_end;
                    }
                }

                str
            }
        }
    }
}

impl Token {
    pub(crate) fn get(&self, client: &clients::sysinfo::Client, interval: Interval) -> f64 {
        let get = |value: Value| value.get(self.prefix);
        let apply = |set: ValueSet| set.apply(&self.function, self.prefix);

        match self.token {
            TokenType::CpuFrequency => apply(client.cpu_frequency()),
            TokenType::CpuPercent => apply(client.cpu_percent()),
            TokenType::MemoryFree => get(client.memory_free()),
            TokenType::MemoryAvailable => get(client.memory_available()),
            TokenType::MemoryTotal => get(client.memory_total()),
            TokenType::MemoryUsed => get(client.memory_used()),
            TokenType::MemoryPercent => get(client.memory_percent()),
            TokenType::SwapFree => get(client.swap_free()),
            TokenType::SwapTotal => get(client.swap_total()),
            TokenType::SwapUsed => get(client.swap_used()),
            TokenType::SwapPercent => get(client.swap_percent()),
            TokenType::TempC => apply(client.temp_c()),
            TokenType::TempF => apply(client.temp_f()),
            TokenType::DiskFree => apply(client.disk_free()),
            TokenType::DiskTotal => apply(client.disk_total()),
            TokenType::DiskUsed => apply(client.disk_used()),
            TokenType::DiskPercent => apply(client.disk_percent()),
            TokenType::DiskRead => apply(client.disk_read(interval)),
            TokenType::DiskWrite => apply(client.disk_write(interval)),
            TokenType::NetDown => apply(client.net_down(interval)),
            TokenType::NetUp => apply(client.net_up(interval)),
            TokenType::LoadAverage1 => get(client.load_average_1()),
            TokenType::LoadAverage5 => get(client.load_average_5()),
            TokenType::LoadAverage15 => get(client.load_average_15()),
            TokenType::Uptime => get(client.uptime()),
        }
    }
}
