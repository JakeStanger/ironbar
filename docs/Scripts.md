There are various places inside the configuration (other than the `script` module)
that allow script input to dynamically set values.

Scripts are passed to `sh -c`.

Three types of scripts exist: polling, oneshot and watching:

- **Polling** scripts will run and wait for exit.
  Normally they will repeat this at an interval, hence the name, although in some cases they may only run on a user
  event.
  If the script exited code 0, the `stdout` will be used. Otherwise, `stderr` will be printed to the log.
- **Oneshot** scripts are a variant of polling scripts. 
  They wait for script to exit, and may do something with the output, but are only fired by user events instead of the interval.
  Generally options that accept oneshot scripts do not support the other types.
- **Watching** scripts start a long-running process. Every time the process writes to `stdout`, the last line is captured
  and used.

One should prefer to use watch-mode where possible, as it removes the overhead of regularly spawning processes.
That said, there are some cases which only support polling. These are indicated by `Script [polling]` as the option
type.

## Writing script configs

There are two available config formats for scripts, shorthand as a string, or longhand as an object.
Shorthand can be used in all cases, but there are some cases (such as embedding scripts inside strings) where longhand
cannot be used.

In both formats, `mode` is one of `poll` or `watch` and `interval` is the number of milliseconds to wait between
spawning the script.

Both `mode` and `interval` are optional and can be excluded to fall back to their defaults of `poll` and `5000`
respectively.

For oneshot scripts, both the mode and interval are ignored.

### Shorthand (string)

Shorthand scripts should be written in the format:

```
mode:interval:script
```

For example:

```
poll:5000:uptime -p | cut -d ' ' -f2-
```

#### Embedding

Some string config options support "embedding scripts". This allows you to mix static/dynamic content.
An example of this is the common `tooltip` option.

Scripts can be embedded in these cases using `{{double braces}}` and the shorthand syntax:

```json
"Up: {{30000:uptime -p | cut -d ' ' -f2-}}"
```

### Longhand (object)

An object consisting of the `cmd` key and optionally the `mode` and/or `interval` keys.

<details>
<summary>JSON</summary>

```json
{
  "mode": "poll",
  "interval": 5000,
  "cmd": "uptime -p | cut -d ' ' -f2-"
}
```
</details>

<details>
<summary>YAML</summary>

```yaml
mode: poll
interval: 5000
cmd: "uptime -p | cut -d ' ' -f2-"
```
</details>

<details>
<summary>YAML</summary>

```toml
mode = "poll"
interval = 5000
cmd = "uptime -p | cut -d ' ' -f2-"
```
</details>

<details>
<summary>Corn</summary>

```corn
{
  mode = "poll"
  interval = 5000
  cmd = "uptime -p | cut -d ' ' -f2-"
}
```
</details>