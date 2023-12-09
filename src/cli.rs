use crate::ipc::commands::Command;
use crate::ipc::responses::Response;
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

pub fn handle_response(response: Response) {
    match response {
        Response::Ok => println!("ok"),
        Response::OkValue { value } => println!("ok\n{value}"),
        Response::Err { message } => eprintln!("error\n{}", message.unwrap_or_default()),
    }
}
