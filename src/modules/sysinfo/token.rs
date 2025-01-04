use crate::clients::sysinfo::{Function, Prefix};

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

#[derive(Debug, Clone)]
pub struct Token {
    pub token: TokenType,
    pub function: Function,
    pub prefix: Prefix,
    pub formatting: Formatting,
}

#[derive(Debug, Clone)]
pub enum Part {
    Static(String),
    Token(Token),
}

#[derive(Debug, Clone, Copy)]
pub struct Formatting {
    pub width: usize,
    pub fill: char,
    pub align: Alignment,
    pub precision: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}
