//! Shared CLI argument definitions for the `ironbar` bar and `ironbar-cli` binaries.

use clap::{CommandFactory, FromArgMatches, ValueEnum};
use ironbar_core::config::ConfigLocation;
use ironbar_ipc_types::Command;

/// Re-exported so dependents can call [`Args::parse`]
/// without a direct `clap` dependency.
pub use clap::Parser;

/// Name and version are intentionally not derived:
/// `#[command(version)]` would embed *this* crate's name/version.
/// The CLI supplies its own at runtime via [`Args::command_with_version`]
/// or [`Args::parse_with_version`]; the bar parses without a version.
#[derive(Parser, Debug)]
#[command(name = "ironbar")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Print the config JSON schema to `stdout`
    /// and exit.
    #[cfg(feature = "schema")]
    #[arg(long("print-schema"))]
    pub print_schema: bool,

    /// Print shell completions to `stdout`
    /// and exit.
    #[cfg(feature = "completions")]
    #[arg(long("print-completions"))]
    pub print_completions: Option<Shell>,

    /// Print debug information to stderr.
    #[arg(long)]
    pub debug: bool,

    /// Specify the path to the config file to use.
    #[arg(short('c'), long, env = "IRONBAR_CONFIG")]
    pub config: Option<ConfigLocation>,

    #[arg(short('t'), long, env = "IRONBAR_CSS")]
    pub theme: Option<ConfigLocation>,

    /// Validate the loaded config and CSS path, and exit immediately.
    /// Exits with code `0` if config is valid, or `1` if invalid.
    /// Pass arg a second time to fail on warnings.
    ///
    /// Any errors or warnings are logged regardless of mode.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub validate_config: u8,

    /// Format to output the response as.
    #[arg(short, long)]
    pub format: Option<Format>,

    /// `bar_id` argument passed by `swaybar_command`.
    /// Not used.
    #[arg(short('b'), hide(true))]
    sway_bar_id: Option<String>,
}

impl Args {
    /// Builds the clap command with the given version string,
    /// which also enables the `-V`/`--version` flag.
    pub fn command_with_version(version: &'static str) -> clap::Command {
        Self::command().version(version)
    }

    /// Parses from `std::env::args_os`, using the given version string.
    pub fn parse_with_version(version: &'static str) -> Self {
        let matches = Self::command_with_version(version).get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
    }
}

#[derive(Debug, Default, ValueEnum, Clone, Copy, Eq, PartialEq)]
pub enum Format {
    #[default]
    Plain,
    Json,
}

#[cfg(feature = "completions")]
#[derive(Debug, ValueEnum, Clone, Copy, Eq, PartialEq)]
pub enum Shell {
    Bash,
    Elvish,
    Zsh,
    Fish,
    Powershell,
}
