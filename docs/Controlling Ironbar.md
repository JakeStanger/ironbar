Ironbar includes a simple IPC server which can be used to control it programmatically at runtime.

It also includes a command line interface, which can be used for interacting with the IPC server.

# CLI

This is shipped as part of the `ironbar` binary. To view commands, you can use `ironbar --help`. 
You can also view help per sub-command or command, for example using `ironbar var --help` or `ironbar var set --help`.

The CLI supports plaintext and JSON output. Plaintext will:

- Print `ok` for empty success responses
- Print the returned body for each success response
  - Some commands act on multiple objects, in which case the CLI will print one line for each body.
- Print `error` to followed by the error on the next line for error responses. This is printed to `stderr`.

Example:

```shell
$ ironbar var set subject world
ok

$ ironbar var get subject
world

$ ironbar var get foo
error
Variable not found
```

All error responses will cause the CLI to exit code 3.

# IPC

The server listens on a Unix socket. 
The path is printed on startup, and can usually be found at `/run/user/$UID/ironbar-ipc.sock`.

Commands and responses are sent as JSON objects.

Commands will have a `command` key, and a `subcommand` key when part of a sub-command.

The message buffer is currently limited to `1024` bytes. 
Particularly large messages will be truncated or cause an error.

The full spec can be found below.

## Libraries

- [Luajit](https://github.com/A-Cloud-Ninja/ironbar-ipc-luajit) - Maintained by [@A-Cloud-Ninja](https://github.com/A-Cloud-Ninja)

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

### `load_css`

Loads an additional CSS stylesheet, with hot-reloading enabled.

Responds with `ok` if the stylesheet exists, otherwise `error`.

```json
{
  "command": "load_css",
  "path": "/path/to/style.css"
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
