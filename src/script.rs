use crate::channels::AsyncSenderExt;
use crate::spawn;
use color_eyre::eyre::WrapErr;
use color_eyre::{Report, Result};
use serde::Deserialize;
use std::cmp::min;
use std::fmt::{Display, Formatter};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, trace, warn};

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ScriptInput {
    String(String),
    Struct(Script),
}

#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ScriptMode {
    Poll,
    Watch,
}

#[derive(Debug, Clone)]
pub enum OutputStream {
    Stdout(String),
    Stderr(String),
}

impl From<&str> for ScriptMode {
    fn from(str: &str) -> Self {
        match str {
            "poll" | "p" => Self::Poll,
            "watch" | "w" => Self::Watch,
            _ => {
                warn!("Invalid script mode: '{str}', falling back to polling");
                Self::Poll
            }
        }
    }
}

impl Default for ScriptMode {
    fn default() -> Self {
        Self::Poll
    }
}

impl Display for ScriptMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Poll => "poll",
                Self::Watch => "watch",
            }
        )
    }
}

impl ScriptMode {
    fn try_parse(str: &str) -> Result<Self> {
        match str {
            "poll" | "p" => Ok(Self::Poll),
            "watch" | "w" => Ok(Self::Watch),
            _ => Err(Report::msg(format!("Invalid script mode: {str}"))),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Script {
    #[serde(default = "ScriptMode::default")]
    pub(crate) mode: ScriptMode,
    pub cmd: String,
    #[serde(default = "default_interval")]
    pub(crate) interval: u64,
}

const fn default_interval() -> u64 {
    5000
}

impl Default for Script {
    fn default() -> Self {
        Self {
            mode: ScriptMode::default(),
            interval: default_interval(),
            cmd: String::new(),
        }
    }
}

impl From<ScriptInput> for Script {
    fn from(input: ScriptInput) -> Self {
        match input {
            ScriptInput::String(string) => Self::from(string.as_str()),
            ScriptInput::Struct(script) => script,
        }
    }
}

#[derive(Debug)]
enum ScriptInputToken {
    Mode(ScriptMode),
    Interval(u64),
    Cmd(String),
}

#[derive(Debug, Copy, Clone)]
enum CurrentToken {
    Mode,
    Interval,
    Cmd,
}

impl From<&str> for Script {
    fn from(str: &str) -> Self {
        let mut script = Self::default();
        let mut tokens = vec![];

        let mut current_state = CurrentToken::Mode;

        let mut chars = str.chars().collect::<Vec<_>>();
        while !chars.is_empty() {
            let char = chars[0];

            let parse_res = match current_state {
                CurrentToken::Mode => {
                    current_state = CurrentToken::Interval;

                    if matches!(char, 'p' | 'w') {
                        let mode_str = chars.iter().take_while(|&c| c != &':').collect::<String>();
                        let len = mode_str.len();

                        let token = ScriptMode::try_parse(&mode_str).ok();
                        token.map(|token| (ScriptInputToken::Mode(token), len))
                    } else {
                        None
                    }
                }
                CurrentToken::Interval => {
                    current_state = CurrentToken::Cmd;

                    if char.is_ascii_digit() {
                        let interval_str = chars
                            .iter()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>();
                        let len = interval_str.len();

                        let token = interval_str.parse::<u64>().ok();
                        token.map(|token| (ScriptInputToken::Interval(token), len))
                    } else {
                        None
                    }
                }
                CurrentToken::Cmd => {
                    let cmd_str = chars.iter().take_while(|_| true).collect::<String>();
                    let len = cmd_str.len();
                    Some((ScriptInputToken::Cmd(cmd_str), len))
                }
            };

            if let Some((token, skip)) = parse_res {
                tokens.push(token);
                chars.drain(..min(skip + 1, chars.len())); // skip 1 extra for colon
            }
        }

        for token in tokens {
            match token {
                ScriptInputToken::Mode(mode) => script.mode = mode,
                ScriptInputToken::Interval(interval) => script.interval = interval,
                ScriptInputToken::Cmd(cmd) => script.cmd = cmd,
            }
        }

        script
    }
}

impl Script {
    pub fn new_polling(input: ScriptInput) -> Self {
        let mut script = Self::from(input);
        script.mode = ScriptMode::Poll;
        script
    }

    /// Runs the script, passing `args` if provided.
    /// Runs `f`, passing the output stream and whether the command returned 0.
    pub async fn run<F>(&self, args: Option<&[String]>, callback: F)
    where
        F: Fn(OutputStream, bool),
    {
        loop {
            match self.mode {
                ScriptMode::Poll => match self.get_output(args).await {
                    Ok(output) => callback(output.0, output.1),
                    Err(err) => error!("{err:?}"),
                },
                ScriptMode::Watch => match self.spawn() {
                    Ok(mut rx) => {
                        while let Some(msg) = rx.recv().await {
                            callback(msg, true);
                        }
                    }
                    Err(err) => error!("{err:?}"),
                },
            };

            sleep(tokio::time::Duration::from_millis(self.interval)).await;
        }
    }

    /// Attempts to execute a given command,
    /// waiting for it to finish.
    /// If the command returns status 0,
    /// the `stdout` is returned.
    /// Otherwise, an `Err` variant
    /// containing the `stderr` is returned.
    pub async fn get_output(&self, args: Option<&[String]>) -> Result<(OutputStream, bool)> {
        let mut args_list = vec!["-c", &self.cmd];

        if let Some(args) = args {
            args_list.extend(args.iter().map(String::as_str));
        }

        debug!("Running sh with args: {args_list:?}");

        let output = Command::new("/bin/sh")
            .args(&args_list)
            .output()
            .await
            .wrap_err("Failed to get script output")?;

        trace!("Script output with args: {output:?}");

        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)
                .map(|output| output.trim().to_string())
                .wrap_err("Script stdout not valid UTF-8")?;

            debug!("sending stdout: '{stdout}'");

            Ok((OutputStream::Stdout(stdout), true))
        } else {
            let stderr = String::from_utf8(output.stderr)
                .map(|output| output.trim().to_string())
                .wrap_err("Script stderr not valid UTF-8")?;

            debug!("sending stderr: '{stderr}'");

            Ok((OutputStream::Stderr(stderr), false))
        }
    }

    /// Spawns a long-running process.
    /// Returns a `mpsc::Receiver` that sends a message
    /// every time a new line is written to `stdout` or `stderr`.
    pub fn spawn(&self) -> Result<mpsc::Receiver<OutputStream>> {
        let mut handle = Command::new("/bin/sh")
            .args(["-c", &self.cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;

        debug!("Spawned a long-running process for '{}'", self.cmd);
        trace!("Handle: {:?}", handle);

        let mut stdout_lines = BufReader::new(
            handle
                .stdout
                .take()
                .expect("Failed to take script handle stdout"),
        )
        .lines();

        let mut stderr_lines = BufReader::new(
            handle
                .stderr
                .take()
                .expect("Failed to take script handle stderr"),
        )
        .lines();

        let (tx, rx) = mpsc::channel(32);

        spawn(async move {
            loop {
                select! {
                    _ = handle.wait() => break,
                    Ok(Some(line)) = stdout_lines.next_line() => {
                        debug!("sending stdout line: '{line}'");
                        tx.send_expect(OutputStream::Stdout(line)).await;
                    }
                    Ok(Some(line)) = stderr_lines.next_line() => {
                        debug!("sending stderr line: '{line}'");
                        tx.send_expect(OutputStream::Stderr(line)).await;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Executes the script in oneshot mode,
    /// meaning it is not awaited and output cannot be captured.
    ///
    /// If the script errors, this is logged.
    ///
    /// This has some overhead,
    /// as the script has to be cloned to the thread.
    ///
    pub fn run_as_oneshot(&self, args: Option<&[String]>) {
        let script = self.clone();
        let args = args.map(<[String]>::to_vec);

        spawn(async move {
            match script.get_output(args.as_deref()).await {
                Ok((OutputStream::Stderr(out), _)) => error!("{out}"),
                Err(err) => error!("{err:?}"),
                _ => {}
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let cmd = "echo 'hello'";
        let script = Script::from(cmd);

        assert_eq!(script.cmd, cmd);
        assert_eq!(script.interval, default_interval());
        assert_eq!(script.mode, ScriptMode::default());
    }

    #[test]
    fn test_parse_full() {
        let cmd = "echo 'hello'";
        let mode = ScriptMode::Watch;
        let interval = 300;

        let full_cmd = format!("{mode}:{interval}:{cmd}");
        let script = Script::from(full_cmd.as_str());

        assert_eq!(script.cmd, cmd);
        assert_eq!(script.mode, mode);
        assert_eq!(script.interval, interval);
    }

    #[test]
    fn test_parse_interval_and_cmd() {
        let cmd = "echo 'hello'";
        let interval = 300;

        let full_cmd = format!("{interval}:{cmd}");
        let script = Script::from(full_cmd.as_str());

        assert_eq!(script.cmd, cmd);
        assert_eq!(script.interval, interval);
        assert_eq!(script.mode, ScriptMode::default());
    }

    #[test]
    fn test_parse_mode_and_cmd() {
        let cmd = "echo 'hello'";
        let mode = ScriptMode::Watch;

        let full_cmd = format!("{mode}:{cmd}");
        let script = Script::from(full_cmd.as_str());

        assert_eq!(script.cmd, cmd);
        assert_eq!(script.interval, default_interval());
        assert_eq!(script.mode, mode);
    }

    #[test]
    fn test_parse_cmd_with_colon() {
        let cmd = "uptime | awk '{print \"Uptime: \" $1}'";
        let script = Script::from(cmd);

        assert_eq!(script.cmd, cmd);
        assert_eq!(script.interval, default_interval());
        assert_eq!(script.mode, ScriptMode::default());
    }

    #[test]
    fn test_no_cmd() {
        let mode = ScriptMode::Watch;
        let interval = 300;

        let full_cmd = format!("{mode}:{interval}");
        let script = Script::from(full_cmd.as_str());

        assert_eq!(script.cmd, ""); // TODO: Probably better handle this case
        assert_eq!(script.interval, interval);
        assert_eq!(script.mode, mode);
    }
}
