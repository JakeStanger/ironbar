// Importing from Ironbar modules brings in lots of things not used by the build script
// we can just globally suppress those.
#![allow(unused, dead_code)]

#[path = "src/cli.rs"]
mod cli;

#[path = "src/error.rs"]
mod error;

#[path = "src/ipc"]
mod ipc {
    #[path = "commands.rs"]
    mod commands;

    #[path = "responses.rs"]
    mod responses;

    pub use commands::Command;
    pub use responses::Response;
}

use clap::Command;
use clap::CommandFactory;
use clap_complete::generate_to;
use clap_complete::Shell::{Bash, Fish, Zsh};
use cli::Args;
use std::fs;
use std::path::PathBuf;

const NAME: &str = "ironbar";

fn generate_shell_completions(mut cmd: Command) -> std::io::Result<()> {
    const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
    let comp_dir = PathBuf::from(MANIFEST_DIR).join("target/completions");

    fs::create_dir_all(&comp_dir)?;

    for shell in [Bash, Fish, Zsh] {
        generate_to(shell, &mut cmd, NAME, &comp_dir)?;
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut cmd = Args::command();
    cmd.set_bin_name(NAME);

    generate_shell_completions(cmd)?;

    Ok(())
}
