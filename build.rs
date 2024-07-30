#[path = "src/cli.rs"]
mod cli;

use clap::Command;
use clap::CommandFactory;
use clap_complete::generate_to;
use clap_complete::Shell::{Bash, Fish, Zsh};
use clap_mangen::Man;
use cli::Args;
use std::fs;
use std::path::PathBuf;

static NAME: &str = "ironbar";

fn generate_man_pages(cmd: Command) {
    let man_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/man");
    let mut buffer = Vec::default();

    Man::new(cmd.clone()).render(&mut buffer).unwrap();
    fs::create_dir_all(&man_dir).unwrap();
    fs::write(man_dir.join(NAME.to_owned() + ".1"), buffer).unwrap();

    for subcommand in cmd.get_subcommands() {
        let mut buffer = Vec::default();

        Man::new(subcommand.clone()).render(&mut buffer).unwrap();
        fs::write(
            man_dir.join(NAME.to_owned() + "-" + subcommand.get_name() + ".1"),
            buffer,
        )
        .unwrap();

        for subsubcommand in subcommand.get_subcommands() {
            let mut buffer = Vec::default();

            Man::new(subsubcommand.clone()).render(&mut buffer).unwrap();
            fs::write(
                man_dir.join(
                    NAME.to_owned()
                        + "-"
                        + subcommand.get_name()
                        + "-"
                        + subsubcommand.get_name()
                        + ".1",
                ),
                buffer,
            )
            .unwrap();
        }
    }
}

fn generate_shell_completions(mut cmd: Command) {
    let comp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/completions");

    fs::create_dir_all(&comp_dir).unwrap();

    for shell in [Bash, Fish, Zsh] {
        generate_to(shell, &mut cmd, NAME, &comp_dir).unwrap();
    }
}

fn main() {
    let mut cmd = Args::command();
    cmd.set_bin_name(NAME);

    generate_man_pages(cmd.clone());
    generate_shell_completions(cmd);
}
