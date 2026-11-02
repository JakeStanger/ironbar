use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Ok,
    OkValue { value: String },
    Multi { values: Vec<String> },
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
