Displays information about the current SwayNC state such as notification count and DnD.
Clicking the widget opens the SwayNC panel.

![Notifications widget in its closed state showing 3 notifications](https://f.jstanger.dev/github/ironbar/notifications.png)

> [!NOTE]
> This widget requires the [SwayNC](https://github.com/ErikReider/SwayNotificationCenter) 
> daemon to be running to use.

## Configuration

> Type: `notifications`

| Name                | Type      | Default | Description                                                                                            |
|---------------------|-----------|---------|--------------------------------------------------------------------------------------------------------|
| `show_count`        | `boolean` | `true`  | Whether to show the current notification count.                                                        |
| `icons.closed_none` | `string`  | `󰍥`    | Icon to show when the panel is closed, with no notifications.                                          |
| `icons.closed_some` | `string`  | `󱥂`    | Icon to show when the panel is closed, with notifications.                                             |
| `icons.closed_dnd`  | `string`  | `󱅯`    | Icon to show when the panel is closed, with DnD enabled. Takes higher priority than count-based icons. |
| `icons.open_none`   | `string`  | `󰍡`    | Icon to show when the panel is open, with no notifications.                                            |
| `icons.open_some`   | `string`  | `󱥁`    | Icon to show when the panel is open, with notifications.                                               |
| `icons.open_dnd`    | `string`  | `󱅮`    | Icon to show when the panel is open, with DnD enabled. Takes higher priority than count-based icons.   |

> [!NOTE]
> This module does not support module-level [layout options](module-level-options#layout).

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "notifications",
      "show_count": true,
      "icons": {
        "closed_none": "󰍥",
        "closed_some": "󱥂",
        "closed_dnd": "󱅯",
        "open_none": "󰍡",
        "open_some": "󱥁",
        "open_dnd": "󱅮"
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
type = "notifications"
show_count = true

[end.icons]
closed_none = "󰍥"
closed_some = "󱥂"
closed_dnd = "󱅯"
open_none = "󰍡"
open_some = "󱥁"
open_dnd = "󱅮"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: notifications
    show_count: true
    icons:
      closed_none: 󰍥
      closed_some: 󱥂
      closed_dnd: 󱅯
      open_none: 󰍡
      open_some: 󱥁
      open_dnd: 󱅮
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "notifications"
      show_count = true

      icons.closed_none = "󰍥"
      icons.closed_some = "󱥂"
      icons.closed_dnd = "󱅯"
      icons.open_none = "󰍡"
      icons.open_some = "󱥁"
      icons.open_dnd = "󱅮"
    }
  ]
}
```

</details>

## Styling

| Selector                | Description                           |
|-------------------------|---------------------------------------|
| `.notifications`        | Notifications widget button           |
| `.notifications .count` | Notifications count indicator overlay |

For more information on styling, please see the [styling guide](styling-guide).