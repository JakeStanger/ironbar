use std::path::PathBuf;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Return "ok"
    Ping,

    /// Open the GTK inspector
    Inspect,

    /// Reload the config
    Reload,

    /// Set an `ironvar` value.
    /// This creates it if it does not already exist, and updates it if it does.
    /// Any references to this variable are automatically and immediately updated.
    /// Keys and values can be any valid UTF-8 string.
    Set {
        /// Variable key. Can be any alphanumeric ASCII string.
        key: Box<str>,
        /// Variable value. Can be any valid UTF-8 string.
        value: String,
    },

    /// Get the current value of an `ironvar`.
    Get {
        /// Variable key.
        key: Box<str>,
    },

    /// Gets the current value of all `ironvar`s.
    List,

    /// Load an additional CSS stylesheet.
    /// The sheet is automatically hot-reloaded.
    LoadCss {
        /// The path to the sheet.
        path: PathBuf,
    },

    /// Set the visibility of the bar with the given name.
    SetVisible {
        ///Bar name to target.
        bar_name: String,
        /// The visibility status.
        #[arg(short, long)]
        visible: bool,
    },

    /// Get the visibility of the bar with the given name.
    GetVisible {
        /// Bar name to target.
        bar_name: String,
    },

    /// Toggle a popup open/closed.
    /// If opening this popup, and a different popup on the same bar is already open, the other is closed.
    TogglePopup {
        /// The name of the monitor the bar is located on.
        bar_name: String,
        /// The name of the widget.
        name: String,
    },

    /// Open a popup, regardless of current state.
    OpenPopup {
        /// The name of the monitor the bar is located on.
        bar_name: String,
        /// The name of the widget.
        name: String,
    },

    /// Close a popup, regardless of current state.
    ClosePopup {
        /// The name of the monitor the bar is located on.
        bar_name: String,
    },
}
