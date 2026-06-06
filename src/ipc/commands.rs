use clap::ArgAction;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// Sends a ping request to the IPC.
    ///
    /// Responds with `ok`.
    ///
    /// ```json
    /// {
    ///   "command": "ping"
    /// }
    /// ```
    Ping,

    /// Opens the GTK inspector window.
    ///
    /// Responds with `ok`.
    ///
    /// ```json
    /// {
    ///   "command": "inspect"
    /// }
    /// ```
    Inspect,

    /// Restarts the bars, reloading the config in the process.
    ///
    /// The IPC server and main GTK application are untouched.
    ///
    /// Responds with `ok`.
    ///
    /// ```json
    /// {
    ///   "command": "reload"
    /// }
    Reload,

    /// Gets and sets reactive Ironvar values.
    #[command(subcommand)]
    Var(IronvarCommand),

    /// Interacts with a specific bar.
    ///
    /// > [!NOTE]
    /// > If there are multiple bars by the same name,
    /// > the `bar` subcommand will act on all of them
    /// > and return a `multi` response for commands that get a value.
    Bar(BarCommand),

    /// Loads stylesheets and dynamically add/remove classes
    #[command(subcommand)]
    Style(StyleCommand),
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "subcommand", rename_all = "snake_case")]
pub enum IronvarCommand {
    /// Sets an [ironvar](ironvars) value.
    /// This creates it if it does not already exist, and updates it if it does.
    /// Any references to this variable are automatically and immediately updated.
    /// Keys and values can be any valid UTF-8 string.
    ///
    /// Responds with `ok`.
    ///
    /// ```json
    /// {
    ///   "command": "var",
    ///   "subcommand": "set",
    ///   "key": "foo",
    ///   "value": "bar"
    /// }
    Set {
        /// Variable key. Can be any alphanumeric ASCII string.
        key: Box<str>,
        /// Variable value. Can be any valid UTF-8 string.
        value: String,
    },

    /// Gets an [ironvar](ironvars) value.
    ///
    /// Responds with `ok_value` if the value exists, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "var",
    ///   "subcommand": "get",
    ///   "key": "foo"
    /// }
    /// ```
    Get {
        /// Variable key.
        key: Box<str>,
    },

    /// Gets a list of all [ironvar](ironvars) values.
    /// Each key/value pair is on its own `\n` separated newline.
    /// The key and value are separated by a colon and space `: `.
    ///
    /// Responds with `ok_value`.
    ///
    /// ```json
    /// {
    ///   "command": "var",
    ///   "subcommand": "list",
    ///   "namespace: "sysinfo"
    /// }
    /// ```
    List {
        /// Namespace to list variables in.
        /// If omitted, the root namespace is used.
        namespace: Option<Box<str>>,
    },
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct BarCommand {
    /// The name of the bar.
    pub name: String,

    #[command(subcommand)]
    #[serde(flatten)]
    pub subcommand: BarCommandType,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "subcommand", rename_all = "snake_case")]
pub enum BarCommandType {
    // == Visibility == \\
    /// Forces the bar to be shown, regardless of the current visibility state.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "show",
    ///   "name": "bar-123"
    /// }
    /// ```
    Show,

    /// Forces the bar to be hidden, regardless of current visibility state.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "hide",
    ///   "name": "bar-123"
    /// }
    /// ```
    Hide,

    /// Sets the bar's visibility state via an argument.
    ///
    /// Responds with `ok` if the bar exists, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "set_visible",
    ///   "name": "bar-123",
    ///   "visible": true
    /// }
    /// ```
    SetVisible {
        /// The new visibility state.
        #[clap(
            num_args(1),
            require_equals(true),
            action = ArgAction::Set,
        )]
        visible: bool,
    },

    /// Toggles the current visibility state between shown and hidden.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "toggle_visible",
    ///   "name": "bar-123"
    /// }
    /// ```
    ToggleVisible,

    /// Gets the bar's visibility state.
    ///
    /// Responds with `ok_value` and the visibility (`true`/`false`)
    /// if the bar exists, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "get_visible",
    ///   "name": "bar-123"
    /// }
    /// ```
    GetVisible,

    // == Popup visibility == \\
    /// Opens a popup, regardless of current state.
    /// If opening this popup, and a different popup on the same bar is already open,
    /// the other is closed.
    ///
    /// Responds with `ok` if the bar and widget exist, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "show_popup",
    ///   "name": "bar-123",
    ///   "widget_name": "clock"
    /// }
    /// ```
    ShowPopup {
        /// The configured name of the widget.
        widget_name: String,
    },

    /// Closes a popup, regardless of current state.
    ///
    /// Responds with `ok` if the bar and widget exist, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "hide_popup",
    ///   "bar_name": "bar-123"
    /// }
    /// ```
    HidePopup,

    /// Sets the popup's visibility state via an argument.
    /// If opening this popup, and a different popup on the same bar is already open,
    /// the other is closed.
    ///
    /// Responds with `ok` if the bar and widget exist, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "set_popup_visible",
    ///   "name": "bar-123",
    ///   "widget_name": "clock",
    ///   "visible": true
    /// }
    /// ```
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

    /// Toggles a popup open/closed.
    /// If opening this popup, and a different popup on the same bar is already open,
    /// the other is closed.
    ///
    /// Responds with `ok` if the bar and widget exist, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "toggle_popup",
    ///   "bar_name": "bar-123",
    ///   "widget_name": "clock"
    /// }
    /// ```
    TogglePopup {
        /// The configured name of the widget.
        widget_name: String,
    },

    /// Gets the popup's current visibility state.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "get_popup_visible",
    ///   "bar_name": "bar-123"
    /// }
    /// ```
    GetPopupVisible,

    // == Exclusivity == \\
    /// Sets whether the bar reserves an exclusive zone.
    ///
    /// ```json
    /// {
    ///   "command": "bar",
    ///   "subcommand": "set_exclusive",
    ///   "bar_name": "bar-123"
    ///   "exclusive": true
    /// }
    /// ```
    SetExclusive {
        #[clap(
            num_args(1),
            require_equals(true),
            action = ArgAction::Set,
        )]
        exclusive: bool,
    },
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "subcommand", rename_all = "snake_case")]
pub enum StyleCommand {
    /// Loads an additional CSS stylesheet.
    /// The sheet is automatically hot-reloaded.
    ///
    /// Responds with `ok` if the stylesheet exists, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "style",
    ///   "subcommand": "load_css",
    ///   "path": "/path/to/style.css"
    /// }
    /// ```
    LoadCss {
        /// The path to the sheet.
        path: PathBuf,
    },

    /// Adds a CSS class `name` to all modules
    /// matching `module_name`.
    /// If the module also has a popup,
    /// the class is added to the top container.
    ///
    /// Response with `ok` if at least one module is found, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "style",
    ///   "subcommand": "add_class",
    ///   "module_name": "clock",
    ///   "name": "night"
    /// }
    /// ```
    AddClass {
        /// The name of the module to target.
        module_name: String,
        /// The class name to add.
        name: String,
    },

    /// Removes a CSS class `name` from all modules
    /// matching `module_name`.
    /// If the module also has a popup,
    /// the class is added to the top container.
    ///
    /// Response with `ok` if at least one module is found, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "style",
    ///   "subcommand": "remove_class",
    ///   "module_name": "clock",
    ///   "name": "night"
    /// }
    /// ```
    RemoveClass {
        /// The name of the module to target.
        module_name: String,
        /// The class name to remove.
        name: String,
    },

    /// Toggles a CSS class `name` on all modules
    /// matching `module_name`.
    /// If the module also has a popup,
    /// the class is added to the top container.
    ///
    /// Response with `ok` if at least one module is found, otherwise `error`.
    ///
    /// ```json
    /// {
    ///   "command": "style",
    ///   "subcommand": "toggle_class",
    ///   "module_name": "clock",
    ///   "name": "night"
    /// }
    /// ```
    ToggleClass {
        /// The name of the module to target.
        module_name: String,
        /// The class name to toggle.
        name: String,
    },
}
