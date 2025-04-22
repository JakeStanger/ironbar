> [!IMPORTANT]
> This module is currently only available on Sway and Hyprland.

Displays Sway's current binding mode or [Hyprland's current submap](https://wiki.hyprland.org/Configuring/Binds/#submaps)
in a label. Nothing is displayed if no binding mode is active.

## Configuration

> Type: `bindmode`

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
      "type": "bindmode",
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
type = "bindmode"
truncate = "start"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "bindmode"
    truncate: "start"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "bindmode"
      truncate = "start"
    }
  ]
}
```

</details>

## Styling

| Selector    | Description            |
| ----------- | ---------------------- |
| `.bindmode` | Bind mode label widget |

For more information on styling, please see the [styling guide](styling-guide).
