use crate::script::Script;
use crate::{glib_recv_mpsc, spawn, try_send};
#[cfg(feature = "ipc")]
use crate::{send_async, Ironbar};
use cfg_if::cfg_if;
use serde::Deserialize;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DynamicBool {
    /// Either a script or variable, to be determined.
    Unknown(String),
    Script(Script),
    #[cfg(feature = "ipc")]
    Variable(Box<str>),
}

impl DynamicBool {
    pub fn subscribe<F>(self, mut f: F)
    where
        F: FnMut(bool) + 'static,
    {
        let value = match self {
            Self::Unknown(input) => {
                if input.starts_with('#') {
                    cfg_if! {
                        if #[cfg(feature = "ipc")] {
                            Self::Variable(input.into())
                        } else {
                            Self::Unknown(input)
                        }
                    }
                } else {
                    let script = Script::from(input.as_str());
                    Self::Script(script)
                }
            }
            _ => self,
        };

        let (tx, rx) = mpsc::channel(32);

        glib_recv_mpsc!(rx, val => f(val));

        spawn(async move {
            match value {
                DynamicBool::Script(script) => {
                    script
                        .run(None, |_, success| {
                            try_send!(tx, success);
                        })
                        .await;
                }
                #[cfg(feature = "ipc")]
                DynamicBool::Variable(variable) => {
                    let variable_manager = Ironbar::variable_manager();

                    let variable_name = variable[1..].into(); // remove hash
                    let mut rx = crate::write_lock!(variable_manager).subscribe(variable_name);

                    while let Ok(value) = rx.recv().await {
                        let has_value = value.is_some_and(|s| is_truthy(&s));
                        send_async!(tx, has_value);
                    }
                }
                DynamicBool::Unknown(_) => unreachable!(),
            }
        });
    }
}

/// Check if a string ironvar is 'truthy',
/// i.e should be evaluated to true.
///
/// This loosely follows the common JavaScript cases.
#[cfg(feature = "ipc")]
fn is_truthy(string: &str) -> bool {
    !(string.is_empty() || string == "0" || string == "false")
}
