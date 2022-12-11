#[repr(i32)]
pub enum ExitCode {
    GtkDisplay = 1,
    CreateBars = 2,
    Config = 3,
}

pub const ERR_OUTPUTS: &str = "GTK and Sway are reporting a different set of outputs - this is a severe bug and should never happen";
pub const ERR_MUTEX_LOCK: &str = "Failed to get lock on Mutex";
pub const ERR_READ_LOCK: &str = "Failed to get read lock";
pub const ERR_WRITE_LOCK: &str = "Failed to get write lock";
pub const ERR_CHANNEL_SEND: &str = "Failed to send message to channel";
pub const ERR_CHANNEL_RECV: &str = "Failed to receive message from channel";
