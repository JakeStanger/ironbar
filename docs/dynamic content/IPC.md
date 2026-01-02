The server listens on a Unix socket.
The path is printed on startup, and can usually be found at `/run/user/$UID/ironbar-ipc.sock`.

Commands and responses are sent as JSON objects.
The JSON should be minified and must NOT contain any `\n` characters.

Commands will have a `command` key, and a `subcommand` key when part of a sub-command.

The full spec can be found below.

## Commands

%{properties:ipc:root}%

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
