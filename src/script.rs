use crate::send_async;
use color_eyre::eyre::WrapErr;
use color_eyre::{Report, Result};
use serde::Deserialize;
use std::fmt::{Display, Formatter};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio::{select, spawn};
use tracing::{error, warn};

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ScriptInput {
    String(String),
    Struct(Script),
}

#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
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
                ScriptMode::Poll
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
                ScriptMode::Poll => "poll",
                ScriptMode::Watch => "watch",
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
    Colon,
}

impl From<&str> for Script {
    fn from(str: &str) -> Self {
        let mut script = Self::default();
        let mut tokens = vec![];

        let mut chars = str.chars().collect::<Vec<_>>();
        while !chars.is_empty() {
            let char = chars[0];

            let (token, skip) = match char {
                ':' => (ScriptInputToken::Colon, 1),
                // interval
                '0'..='9' => {
                    let interval_str = chars
                        .iter()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>();

                    let interval = interval_str.parse::<u64>().unwrap_or_else(|_| {
                        warn!("Received invalid interval in script string. Falling back to default `5000ms`.");
                        5000
                    });
                    (ScriptInputToken::Interval(interval), interval_str.len())
                }
                // watching or polling
                'w' | 'p' => {
                    let mode_str = chars.iter().take_while(|&c| c != &':').collect::<String>();
                    let len = mode_str.len();

                    let token = ScriptMode::try_parse(&mode_str)
                        .map_or(ScriptInputToken::Cmd(mode_str), |mode| {
                            ScriptInputToken::Mode(mode)
                        });

                    (token, len)
                }
                _ => {
                    let cmd_str = chars.iter().take_while(|_| true).collect::<String>();
                    let len = cmd_str.len();
                    (ScriptInputToken::Cmd(cmd_str), len)
                }
            };

            tokens.push(token);
            chars.drain(..skip);
        }

        for token in tokens {
            match token {
                ScriptInputToken::Mode(mode) => script.mode = mode,
                ScriptInputToken::Interval(interval) => script.interval = interval,
                ScriptInputToken::Cmd(cmd) => script.cmd = cmd,
                ScriptInputToken::Colon => {}
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

    pub async fn run<F>(&self, callback: F)
    where
        F: Fn((OutputStream, bool)),
    {
        loop {
            match self.mode {
                ScriptMode::Poll => match self.get_output().await {
                    Ok(output) => callback(output),
                    Err(err) => error!("{err:?}"),
                },
                ScriptMode::Watch => match self.spawn().await {
                    Ok(mut rx) => {
                        while let Some(msg) = rx.recv().await {
                            callback((msg, true));
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
    pub async fn get_output(&self) -> Result<(OutputStream, bool)> {
        let output = Command::new("sh")
            .args(["-c", &self.cmd])
            .output()
            .await
            .wrap_err("Failed to get script output")?;

        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)
                .map(|output| output.trim().to_string())
                .wrap_err("Script stdout not valid UTF-8")?;

            Ok((OutputStream::Stdout(stdout), true))
        } else {
            let stderr = String::from_utf8(output.stderr)
                .map(|output| output.trim().to_string())
                .wrap_err("Script stderr not valid UTF-8")?;

            Ok((OutputStream::Stderr(stderr), false))
        }
    }

    pub async fn spawn(&self) -> Result<mpsc::Receiver<OutputStream>> {
        let mut handle = Command::new("sh")
            .args(["-c", &self.cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;

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
                        send_async!(tx, OutputStream::Stdout(line));
                    }
                    Ok(Some(line)) = stderr_lines.next_line() => {
                        send_async!(tx, OutputStream::Stderr(line));
                    }
                }
            }
        });

        Ok(rx)
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
