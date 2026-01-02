> [!NOTE]
> This widget requires the [SwayNC](https://github.com/ErikReider/SwayNotificationCenter)
> daemon to be running to use.

Displays information about the current SwayNC state such as notification count and DnD.
Clicking the widget opens the SwayNC panel.

![Notifications widget in its closed state showing 2 notifications](https://f.jstanger.dev/github/ironbar/modules/notifications.png)

## Example

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

## Configuration

> Type: `notifications`

> [!NOTE]
> This module does not support module-level [layout options](module-level-options#layout).

%{properties}%

## Styling

| Selector                 | Description                           |
|--------------------------|---------------------------------------|
| `.notifications`         | Notifications widget container        |
| `.notifications .button` | Notifications widget button           |
| `.notifications .count`  | Notifications count indicator overlay |

For more information on styling, please see the [styling guide](styling-guide).