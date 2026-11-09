use crate::CLI_VERSION;
use ironbar_args::{Args, Shell};
use std::io;

pub fn generate(shell: Shell) {
    let mut cmd = Args::command_with_version(CLI_VERSION);
    let name = cmd.get_name().to_string();
    clap_complete::generate(to_clap_shell(shell), &mut cmd, name, &mut io::stdout());
}

/// Maps the shared `Shell` arg onto `clap_complete`'s type.
/// A free function rather than `From` due to the orphan rule.
fn to_clap_shell(shell: Shell) -> clap_complete::Shell {
    use ironbar_args::Shell;

    match shell {
        Shell::Bash => clap_complete::Shell::Bash,
        Shell::Elvish => clap_complete::Shell::Elvish,
        Shell::Zsh => clap_complete::Shell::Zsh,
        Shell::Fish => clap_complete::Shell::Fish,
        Shell::Powershell => clap_complete::Shell::PowerShell,
    }
}
