> [!NOTE]
> This module requires your user is in the `input` group.

> [!IMPORTANT]
> The keyboard layout feature is only available on Sway and Hyprland.

Displays the toggle state of the capslock, num lock and scroll lock keys, and the current keyboard layout.

![Screenshot of keys widget](https://f.jstanger.dev/github/ironbar/keys.png)

## Configuration

> Type: `keys`

| Name               | Type                           | Default | Description                                                                                                               |
| ------------------ | ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------- |
| `show_caps`        | `boolean`                      | `true`  | Whether to show capslock indicator.                                                                                       |
| `show_num`         | `boolean`                      | `true`  | Whether to show num lock indicator.                                                                                       |
| `show_scroll`      | `boolean`                      | `true`  | Whether to show scroll lock indicator.                                                                                    |
| `icon_size`        | `integer`                      | `32`    | Size to render icon at (image icons only).                                                                                |
| `icons.caps_on`    | `string` or [image](images)    | `󰪛`     | Icon to show for enabled capslock indicator.                                                                              |
| `icons.caps_off`   | `string` or [image](images)    | `''`    | Icon to show for disabled capslock indicator.                                                                             |
| `icons.num_on`     | `string` or [image](images)    | ``     | Icon to show for enabled num lock indicator.                                                                              |
| `icons.num_off`    | `string` or [image](images)    | `''`    | Icon to show for disabled num lock indicator.                                                                             |
| `icons.scroll_on`  | `string` or [image](images)    | ``     | Icon to show for enabled scroll lock indicator.                                                                           |
| `icons.scroll_off` | `string` or [image](images)    | `''`    | Icon to show for disabled scroll lock indicator.                                                                          |
| `icons.layout_map` | `Map<string, string or image>` | `{}`    | Map of icons or labels to show for a particular keyboard layout. Layouts use their actual name if not present in the map. |
| `seat`             | `string`                       | `seat0` | ID of the Wayland seat to attach to.                                                                                      |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "keys",
      "show_scroll": false,
      "icons": {
        "caps_on": "󰪛",
        "layout_map": {
          "English (US)": "🇺🇸",
          "Ukrainian": "🇺🇦"
        }
      }
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "keys"
show_scroll = false

[end.icons]
caps_on = "󰪛"

[end.icons.layout_map]
"English (US)" = "🇺🇸"
Ukrainian = "🇺🇦"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: keys
    show_scroll: false
    icons:
      caps_on: 󰪛
      layout_map:
        "English (US)": 🇺🇸
        Ukrainian: 🇺🇦

```

</details>

<details>
<summary>Corn</summary>

```corn
{
end = [ 
        { 
            type = "keys" 
            show_scroll = false 
            icons.caps_on = "󰪛" 
            icons.layout_map.'English (US)' = "🇺🇸"
            icons.layout_map.Ukrainian = "🇺🇦"
        }
    ]
}
```

</details>

## Styling

| Selector               | Description                                |
| ---------------------- | ------------------------------------------ |
| `.keys`                | Keys box container widget.                 |
| `.keys .key`           | Individual key indicator container widget. |
| `.keys .key.enabled`   | Key indicator where key is toggled on.     |
| `.keys .key.caps`      | Capslock key indicator.                    |
| `.keys .key.num`       | Num lock key indicator.                    |
| `.keys .key.scroll`    | Scroll lock key indicator.                 |
| `.keys .key.image`     | Key indicator image icon.                  |
| `.keys .key.text-icon` | Key indicator textual icon.                |
| `.keys .layout`        | Keyboard layout indicator.                 |

For more information on styling, please see the [styling guide](styling-guide).
