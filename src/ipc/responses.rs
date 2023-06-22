use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    Ok,
    Err { message: Option<String> },
}

impl Response {
    /// Creates a new `Response::Error`.
    pub fn error(message: &str) -> Self {
        Self::Err {
            message: Some(message.to_string()),
        }
    }
}
