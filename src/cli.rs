use crate::ipc::commands::Command;
use crate::ipc::responses::Response;
use clap::Parser;
use serde::{Deserialize, Serialize};

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

    /// `bar_id` argument passed by `swaybar_command`.
    /// Not used.
    #[arg(short('b'), hide(true))]
    sway_bar_id: Option<String>,
}

pub fn handle_response(response: Response) {
    match response {
        Response::Ok => println!("ok"),
        Response::OkValue { value } => println!("ok\n{value}"),
        Response::Err { message } => eprintln!("error\n{}", message.unwrap_or_default()),
    }
}
