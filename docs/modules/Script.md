Executes a script and shows the result of `stdout` on a label.
Pango markup is supported.

If you want to be able to embed multiple scripts and/or variables, prefer the [label](label) module.
For more advanced use-cases, use [custom](custom).

## Configuration

> Type: `script`

| Name       | Type                  | Default | Description                                             |
|------------|-----------------------|---------|---------------------------------------------------------|
| `cmd`      | `string`              | `null`  | Path to the script on disk                              |
| `mode`     | `'poll'` or `'watch'` | `poll`  | See [#modes](#modes)                                    |
| `interval` | `number`              | `5000`  | Number of milliseconds to wait between executing script |

### Modes

- Use `poll` to run the script wait for it to exit. On exit, the label is updated to show everything the script wrote to `stdout`.
- Use `watch` to start a long-running script. Every time the script writes to `stdout`, the label is updated to show the latest line.
    Note this does not work for all programs as they may use block-buffering instead of line-buffering when they detect output being piped. 

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "script",
      "cmd": "/home/jake/.local/bin/phone-battery",
      "mode": "poll",
      "interval": 5000
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "script"
cmd = "/home/jake/.local/bin/phone-battery"
mode = "poll"
interval = 5000
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "script"
    cmd: "/home/jake/.local/bin/phone-battery"
    mode: 'poll'
    interval : 5000
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "script"
      cmd = "/home/jake/.local/bin/phone-battery"
      mode = "poll"
      interval = 5000
    }
  ]
}
```

</details>

## Styling

| Selector  | Description         |
|-----------|---------------------|
| `.script` | Script widget label |

For more information on styling, please see the [styling guide](styling-guide).