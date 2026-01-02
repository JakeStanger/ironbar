Ironbar includes a simple [IPC](ipc) server which can be used to control it programmatically at runtime.

It also includes a Command Line Interface, which can be used for interacting with the IPC server.
The CLI is auto-generated from the IPC definition, so always includes all commands.

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
