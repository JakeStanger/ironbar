use crate::client::IpcClient;
use ironbar_args::{Args, Format};
use ironbar_core::error::ExitCode;
use ironbar_ipc_types::{Ipc, Response};
use std::env::args_os;
use std::ffi::OsString;
use std::process::exit;

mod client;

#[cfg(feature = "completions")]
mod completions;
pub mod exec;

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = args_os().collect::<Vec<_>>();

    if should_forward_command(&args) {
        return exec::forward_command(&args);
    }

    let args = Args::parse_with_version(CLI_VERSION);

    #[cfg(feature = "completions")]
    if let Some(shell) = args.print_completions {
        return completions::generate(shell);
    }

    match args.command {
        Some(command) => {
            if args.debug {
                eprintln!("REQUEST: {command:?}");
            }

            let ipc = Ipc::new();
            match ipc.send(command, args.debug) {
                Ok(res) => {
                    if args.debug {
                        eprintln!("RESPONSE: {res:?}");
                    }

                    handle_response(res, args.format.unwrap_or_default());
                }
                Err(err) => {
                    eprintln!("{err:#}");
                    exit(ExitCode::IpcResponseError as i32)
                }
            }
        }
        None => {
            panic!(
                "non-ipc command should be forwarded to main executable - this is a serious bug!"
            )
        }
    }
}

fn should_forward_command(args: &[OsString]) -> bool {
    const COMMANDS: [&str; 6] = ironbar_ipc_types::Command::to_options_array();
    const INTERNAL_FLAGS: [&str; 6] = [
        "--help",
        "-h",
        "--version",
        "-V",
        "help",
        "--print-completions",
    ];

    !args.iter().any(|x| {
        COMMANDS.contains(&x.to_string_lossy().as_ref())
            || INTERNAL_FLAGS.contains(&x.to_string_lossy().as_ref())
    })
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
