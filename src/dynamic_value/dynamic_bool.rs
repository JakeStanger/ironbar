use crate::script::Script;
use crate::send;
#[cfg(feature = "ipc")]
use crate::Ironbar;
use cfg_if::cfg_if;
use glib::Continue;
use serde::Deserialize;
use tokio::spawn;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum DynamicBool {
    /// Either a script or variable, to be determined.
    Unknown(String),
    Script(Script),
    #[cfg(feature = "ipc")]
    Variable(Box<str>),
}

impl DynamicBool {
    pub fn subscribe<F>(self, f: F)
    where
        F: FnMut(bool) -> Continue + 'static,
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

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        rx.attach(None, f);

        spawn(async move {
            match value {
                DynamicBool::Script(script) => {
                    script
                        .run(None, |_, success| {
                            send!(tx, success);
                        })
                        .await;
                }
                #[cfg(feature = "ipc")]
                DynamicBool::Variable(variable) => {
                    let variable_manager = Ironbar::variable_manager();

                    let variable_name = variable[1..].into(); // remove hash
                    let mut rx = crate::write_lock!(variable_manager).subscribe(variable_name);

                    while let Ok(value) = rx.recv().await {
                        let has_value = value.map(|s| is_truthy(&s)).unwrap_or_default();
                        send!(tx, has_value);
                    }
                }
                DynamicBool::Unknown(_) => unreachable!(),
            }
        });
    }
}

/// Check if a string ironvar is 'truthy'
#[cfg(feature = "ipc")]
fn is_truthy(string: &str) -> bool {
    !(string.is_empty() || string == "0" || string == "false")
}
