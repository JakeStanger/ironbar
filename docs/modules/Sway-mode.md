Displays the current sway mode.

## Configuration

> Type: `sway-mode`

| Name                  | Type                                        | Default | Description                                                                                                                                           |
| --------------------- | ------------------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `truncate`            | `'start'` or `'middle'` or `'end'` or `Map` | `null`  | The location of the ellipses and where to truncate text from. Leave null to avoid truncating. Use the long-hand `Map` version if specifying a length. |
| `truncate.mode`       | `'start'` or `'middle'` or `'end'`          | `null`  | The location of the ellipses and where to truncate text from. Leave null to avoid truncating.                                                         |
| `truncate.length`     | `integer`                                   | `null`  | The fixed width (in chars) of the widget. Leave blank to let GTK automatically handle.                                                                |
| `truncate.max_length` | `integer`                                   | `null`  | The maximum number of characters before truncating. Leave blank to let GTK automatically handle.                                                      |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "sway-mode",
      "truncate": "start"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "sway-mode"
truncate = "start"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "sway-mode"
    truncate: "start"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "sway-mode"
      truncate = "start"
    }
  ]
}
```

</details>

## Styling

| Selector | Description  |
| -------- | ------------ |
| `.label` | Label widget |

For more information on styling, please see the [styling guide](styling-guide).
