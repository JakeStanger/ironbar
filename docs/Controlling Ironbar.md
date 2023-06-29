Ironbar includes a simple IPC server which can be used to control it programmatically at runtime.

It also includes a command line interface, which can be used for interacting with the IPC server.

# CLI

This is shipped as part of the `ironbar` binary. To view commands, you can use `ironbar --help`. 
You can also view help per-command, for example using `ironbar set --help`.

Responses are handled by writing their type to stdout, followed by any value starting on the next line.
Error responses are written to stderr in the same format.

Example:

```shell
$ ironbar set subject world
ok

$ ironbar get subject
ok
world
```

# IPC

The server listens on a Unix socket. 
This can usually be found at `/run/user/$UID/ironbar-ipc.sock`.

Commands and responses are sent as JSON objects, denoted by their `type` key.

The message buffer is currently limited to `1024` bytes. 
Particularly large messages will be truncated or cause an error.

## Commands

### `ping`

Sends a ping request to the IPC.

Responds with `ok`.

```json
{
  "type": "ping"
}
```

### `inspect`

Opens the GTK inspector window.

Responds with `ok`.

```json
{
  "type": "inspect"
}
```

### `get`

Gets an [ironvar](ironvars) value.

Responds with `ok_value` if the value exists, otherwise `error`.

```json
{
  "type": "get",
  "key": "foo"
}
```

### `set`

Sets an [ironvar](ironvars) value.

Responds with `ok`.

```json
{
  "type": "set",
  "key": "foo",
  "value": "bar"
}
```

### `load_css`

Loads an additional CSS stylesheet, with hot-reloading enabled.

Responds with `ok` if the stylesheet exists, otherwise `error`.

```json
{
  "type": "load_css",
  "path": "/path/to/style.css"
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

### `error`

The operation failed.

Message is optional.

```json
{
  "type": "error",
  "message": "lorem ipsum"
}
```