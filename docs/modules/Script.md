Executes a script and shows the result of `stdout` on a label.
Pango markup is supported.

## Configuration

> Type: `script`

| Name      | Type     | Default | Description                                             |
|-----------|----------|---------|---------------------------------------------------------|
| `path`    | `string` | `null`  | Path to the script on disk                              |
| `interval` | `number` | `5000`   | Number of milliseconds to wait between executing script |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "script",
      "path": "/home/jake/.local/bin/phone-battery",
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
path = "/home/jake/.local/bin/phone-battery"
interval = 5000
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "script"
    path: "/home/jake/.local/bin/phone-battery"
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
      path = "/home/jake/.local/bin/phone-battery"
      interval = 5000
    }
  ]
}
```

</details>

## Styling

| Selector      | Description         |
|---------------|---------------------|
| `#script`     | Script widget label |