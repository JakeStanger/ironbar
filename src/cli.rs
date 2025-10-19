use crate::config::ConfigLocation;
use crate::error::ExitCode;
use crate::ipc::{Command, Response};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::process::exit;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Print the config JSON schema to `stdout`
    /// and exit.
    #[cfg(feature = "extras")]
    #[arg(long("print-schema"))]
    pub print_schema: bool,

    /// Print shell completions to `stdout`
    /// and exit.
    #[cfg(feature = "extras")]
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

    /// Format to output the response as.
    #[arg(short, long)]
    pub format: Option<Format>,

    /// `bar_id` argument passed by `swaybar_command`.
    /// Not used.
    #[arg(short('b'), hide(true))]
    sway_bar_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, ValueEnum, Clone, Copy, Eq, PartialEq)]
pub enum Format {
    #[default]
    Plain,
    Json,
}

#[cfg(feature = "extras")]
#[derive(Debug, Serialize, Deserialize, ValueEnum, Clone, Copy, Eq, PartialEq)]
pub enum Shell {
    Bash,
    Elvish,
    Zsh,
    Fish,
    Powershell,
}

#[cfg(feature = "extras")]
impl From<Shell> for clap_complete::Shell {
    fn from(value: Shell) -> Self {
        match value {
            Shell::Bash => Self::Bash,
            Shell::Elvish => Self::Elvish,
            Shell::Zsh => Self::Zsh,
            Shell::Fish => Self::Fish,
            Shell::Powershell => Self::PowerShell,
        }
    }
}

pub fn handle_response(response: Response, format: Format) {
    let is_err = matches!(response, Response::Err { .. });

    match format {
        Format::Plain => match response {
            Response::Ok => println!("ok"),
            Response::OkValue { value } => println!("{value}"),
            Response::Multi { values } => println!("{}", values.join("\n")),
            Response::Err { message } => eprintln!("error\n{}", message.unwrap_or_default()),
        },
        Format::Json => println!(
            "{}",
            serde_json::to_string(&response).expect("to be valid json")
        ),
    }

    if is_err {
        exit(ExitCode::IpcResponseError as i32)
    }
}
