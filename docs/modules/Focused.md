> [!NOTE]
> This module requires a `wlroots-based` compositor. It will not work without the [wlr-foreign-toplevel-management](https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1) protocol.

Displays the title and/or icon of the currently focused window.

![Screenshot of focused widget, showing an Ironbar file currently open in RustRover](https://f.jstanger.dev/github/ironbar/modules/focused.png)


## Configuration

> Type: `focused`

| Name                  | Type                                                 | Default | Description                                                                                                                                           |
|-----------------------|------------------------------------------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `show_icon`           | `boolean`                                            | `true`  | Whether to show the app's icon.                                                                                                                       |
| `show_title`          | `boolean`                                            | `true`  | Whether to show the app's title.                                                                                                                      |
| `icon_size`           | `integer`                                            | `32`    | Size of icon in pixels.                                                                                                                               |
| `truncate`            | `'start'` or `'middle'` or `'end'` or `off` or `Map` | `off`   | The location of the ellipses and where to truncate text from. Leave null to avoid truncating. Use the long-hand `Map` version if specifying a length. |
| `truncate.mode`       | `'start'` or `'middle'` or `'end'` or `off`          | `off`   | The location of the ellipses and where to truncate text from. Leave null to avoid truncating.                                                         |
| `truncate.length`     | `integer`                                            | `null`  | The fixed width (in chars) of the widget. Leave blank to let GTK automatically handle.                                                                |
| `truncate.max_length` | `integer`                                            | `null`  | The maximum number of characters before truncating. Leave blank to let GTK automatically handle.                                                      |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "focused",
      "show_icon": true,
      "show_title": true,
      "icon_size": 32,
      "truncate": "end"
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "focused"
show_icon = true
show_title = true
icon_size = 32
truncate = "end"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "focused"
    show_icon: true
    show_title: true
    icon_size: 32
    truncate: "end"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "focused"
      show_icon = true
      show_title = true
      icon_size = 32
      truncate = "end"
    }
  ]
}
```

</details>

## Styling

| Selector          | Description        |
|-------------------|--------------------|
| `.focused`        | Focused widget box |
| `.focused .icon`  | App icon           |
| `.focused .label` | App name           |

For more information on styling, please see the [styling guide](styling-guide).