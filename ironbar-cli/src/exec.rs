use ironbar_core::error::ExitCode;
use std::ffi::OsString;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

pub(crate) fn forward_command(args: &[OsString]) {
    match locate_command() {
        Ok(path) => {
            let err = Command::new(path)
                .arg0("ironbar")
                .args(args.iter().skip(1))
                .exec();

            eprintln!("could not start ironbar: {err:?}");
            exit(ExitCode::ServerExecutableExecError as i32)
        }
        Err(err) => {
            eprintln!("could not start ironbar: {err:?}");
            exit(ExitCode::ServerExecutableNotFound as i32)
        }
    }
}

fn locate_command() -> Result<PathBuf, io::Error> {
    const SERVER_BIN: &str = "ironbard"; // must match bin entry in main cargo.toml
    const ENV_OVERRIDE: &str = "IRONBAR_PATH";

    if let Some(path) = std::env::var_os(ENV_OVERRIDE).map(PathBuf::from) {
        if is_executable(&path) {
            return Ok(path);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "file at IRONBAR_PATH is not executable",
            ));
        }
    }

    let exe = std::env::current_exe();

    let mut candiates = vec![];

    if let Ok(mut path) = exe {
        path.pop();
        candiates.push(path.join(SERVER_BIN));
    }

    for candidate in candiates {
        if is_executable(&candidate) {
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "ironbar binary not found",
    ))
}

fn is_executable(path: &Path) -> bool {
    const EXE_MASK: u32 = 0o111;
    path.metadata()
        .is_ok_and(|p| p.is_file() && p.permissions().mode() & EXE_MASK != 0)
}
