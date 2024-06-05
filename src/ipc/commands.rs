use clap::ArgAction;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// Pong
    Ping,

    /// Open the GTK inspector.
    Inspect,

    /// Reload the config.
    Reload,

    /// Load an additional CSS stylesheet.
    /// The sheet is automatically hot-reloaded.
    LoadCss {
        /// The path to the sheet.
        path: PathBuf,
    },

    /// Get and set reactive Ironvar values.
    #[command(subcommand)]
    Var(IronvarCommand),

    /// Interact with a specific bar.
    Bar(BarCommand),
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[serde(tag = "subcommand", rename_all = "snake_case")]
pub enum IronvarCommand {
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
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct BarCommand {
    /// The name of the bar.
    pub name: String,

    #[command(subcommand)]
    #[serde(flatten)]
    pub subcommand: BarCommandType,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[serde(tag = "subcommand", rename_all = "snake_case")]
pub enum BarCommandType {
    // == Visibility == \\
    /// Force the bar to be shown, regardless of current visibility state.
    Show,
    /// Force the bar to be hidden, regardless of current visibility state.
    Hide,
    /// Set the bar's visibility state via an argument.
    SetVisible {
        /// The new visibility state.
        #[clap(
            num_args(1),
            require_equals(true),
            action = ArgAction::Set,
        )]
        visible: bool,
    },
    /// Toggle the current visibility state between shown and hidden.
    ToggleVisible,
    /// Get the bar's visibility state.
    GetVisible,

    // == Popup visibility == \\
    /// Open a popup, regardless of current state.
    /// If opening this popup, and a different popup on the same bar is already open, the other is closed.
    ShowPopup {
        /// The configured name of the widget.
        widget_name: String,
    },
    /// Close a popup, regardless of current state.
    HidePopup,
    /// Set the popup's visibility state via an argument.
    /// If opening this popup, and a different popup on the same bar is already open, the other is closed.
    SetPopupVisible {
        /// The configured name of the widget.
        widget_name: String,

        #[clap(
            num_args(1),
            require_equals(true),
            action = ArgAction::Set,
        )]
        visible: bool,
    },
    /// Toggle a popup open/closed.
    /// If opening this popup, and a different popup on the same bar is already open, the other is closed.
    TogglePopup {
        /// The configured name of the widget.
        widget_name: String,
    },
    /// Get the popup's current visibility state.
    GetPopupVisible,
}
