use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    /// Return "ok"
    Ping,

    /// Open the GTK inspector
    Inspect,
}

