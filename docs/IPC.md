# IPC

The server listens on a Unix socket.
The path is printed on startup, and can usually be found at `/run/user/$UID/ironbar-ipc.sock`.

Commands and responses are sent as JSON objects.
The JSON should be minified and must NOT contain any `\n` characters.

Commands will have a `command` key, and a `subcommand` key when part of a sub-command.

The full spec can be found below.

## Commands

### `ping`

Sends a ping request to the IPC.

Responds with `ok`.

```json
{
  "command": "ping"
}
```

### `inspect`

Opens the GTK inspector window.

Responds with `ok`.

```json
{
  "command": "inspect"
}
```

### `reload`

Restarts the bars, reloading the config in the process.

The IPC server and main GTK application are untouched.

Responds with `ok`.

```json
{
  "command": "reload"
}
```

### `var`

Subcommand for controlling Ironvars.

#### `get`

Gets an [ironvar](ironvars) value.

Responds with `ok_value` if the value exists, otherwise `error`.

```json
{
  "command": "var",
  "subcommand": "get",
  "key": "foo"
}
```

#### `set`

Sets an [ironvar](ironvars) value.

Responds with `ok`.

```json
{
  "command": "var",
  "subcommand": "set",
  "key": "foo",
  "value": "bar"
}
```

#### `list`

Gets a list of all [ironvar](ironvars) values.

Responds with `ok_value`.

Each key/value pair is on its own `\n` separated newline. The key and value are separated by a colon and space `: `.

```json
{
  "command": "var",
  "subcommand": "list"
}
```

### `bar`

> [!NOTE]
> If there are multiple bars by the same name, the `bar` subcommand will act on all of them and return a `multi` response for commands that get a value.

#### `show`

Forces a bar to be shown, regardless of the current visibility state.

```json
{
  "command": "bar",
  "subcommand": "show",
  "name": "bar-123"
}
```

#### `hide`

Forces a bar to be hidden, regardless of the current visibility state.

```json
{
  "command": "bar",
  "subcommand": "hide",
  "name": "bar-123"
}
```

#### `set_visible`

Sets a bar's visibility to one of shown/hidden.

Responds with `ok` if the bar exists, otherwise `error`.

```json
{
  "command": "bar",
  "subcommand": "set_visible",
  "name": "bar-123",
  "visible": true
}
```

#### `toggle_visible`

Toggles the current visibility state of a bar between shown and hidden.

```json
{
  "command": "bar",
  "subcommand": "toggle_visible",
  "name": "bar-123"
}
```

#### `get_visible`

Gets a bar's visibility.

Responds with `ok_value` and the visibility (`true`/`false`) if the bar exists, otherwise `error`.

```json
{
  "command": "bar",
  "subcommand": "get_visible",
  "name": "bar-123"
}
```

#### `show_popup`

Sets a module's popup open, regardless of its current state.
Since each bar only has a single popup, any open popup on the bar is closed.

Responds with `ok` if the bar and widget exist, otherwise `error`.

```json
{
  "command": "bar",
  "subcommand": "show_popup",
  "name": "bar-123",
  "widget_name": "clock"
}
```

#### `hide_popup`

Sets the popup on a bar closed, regardless of which module it is open for.

Responds with `ok` if the bar and widget exist, otherwise `error`.

```json
{
  "command": "bar",
  "subcommand": "hide_popup",
  "bar_name": "bar-123"
}
```

#### `set_popup_visible`

Sets a popup's visibility to one of shown/hidden.

Responds with `ok` if the bar and widget exist, otherwise `error`.

```json
{
  "command": "bar",
  "subcommand": "set_popup_visible",
  "name": "bar-123",
  "widget_name": "clock",
  "visible": true
}
```

#### `toggle_popup`

Toggles the open/closed state for a module's popup.
Since each bar only has a single popup, any open popup on the bar is closed.

Responds with `ok` if the bar and widget exist, otherwise `error`.

```json
{
  "command": "bar",
  "subcommand": "toggle_popup",
  "bar_name": "bar-123",
  "widget_name": "clock"
}
```

#### `get_popup_visible`

Gets the popup's current visibility state.

```json
{
  "command": "bar",
  "subcommand": "get_popup_visible",
  "bar_name": "bar-123"
}
```

#### `set_exclusive`

Sets whether the bar reserves an exclusive zone.

```json
{
  "command": "bar",
  "subcommand": "set_exclusive",
  "exclusive": true
}
```

### `style`

#### `load_css`

Loads an additional CSS stylesheet, with hot-reloading enabled.

Responds with `ok` if the stylesheet exists, otherwise `error`.

```json
{
  "command": "load_css",
  "path": "/path/to/style.css"
}
```

#### `add_class`

Adds a CSS class to the top-level widget for all modules matching `module_name`.
If the module also has a popup, the class is added to the top container.

Response with `ok` if at least one module is found, otherwise `error`.

```json
{
  "command": "add_class",
  "module_name": "clock",
  "name": "night"
}
```

#### `remove_class`

Removes a CSS class from the top-level widget for all modules matching `module_name`.
If the module also has a popup, the class is added to the top container.

Response with `ok` if at least one module is found, otherwise `error`.

```json
{
  "command": "remove_class",
  "module_name": "clock",
  "name": "night"
}
```

#### `toggle_class`

Toggles a CSS class on the top-level widget for all modules matching `module_name`,
removing it if already present and adding it otherwise.
If the module also has a popup, the class is added to the top container.

Response with `ok` if at least one module is found, otherwise `error`.

```json
{
  "command": "toggle_class",
  "module_name": "clock",
  "name": "night"
}
```

## Responses

### `ok`

The operation completed successfully, with no response data.

```json
{
  "type": "ok"
}
```

### `ok_value`

The operation completed successfully, with response data.

```json
{
  "type": "ok_value",
  "value": "lorem ipsum"
}
```

### `multi`

The operation completed successfully on multiple objects, with response data.

```json
{
  "type": "multi",
  "values": ["lorem ipsum", "dolor sit"]
}
```

### `error`

The operation failed.

Message is optional.

```json
{
  "type": "error",
  "message": "lorem ipsum"
}
```
