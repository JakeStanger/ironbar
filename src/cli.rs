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

    /// Prints the config JSON schema to `stdout`
    /// and exits.
    #[cfg(feature = "schema")]
    #[arg(long("print-schema"))]
    pub print_schema: bool,

    /// Print debug information to stderr
    /// TODO: Make bar follow this too
    #[arg(long)]
    pub debug: bool,

    /// Format to output the response as.
    #[arg(short, long)]
    pub format: Option<Format>,

    /// `bar_id` argument passed by `swaybar_command`.
    /// Not used.
    #[arg(short('b'), hide(true))]
    sway_bar_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, ValueEnum, Clone, Copy)]
pub enum Format {
    #[default]
    Plain,
    Json,
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
