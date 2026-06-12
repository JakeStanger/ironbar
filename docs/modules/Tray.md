Displays a fully interactive icon tray using the KDE `libappindicator` and `com.canonical.dbusmenu` protocols.

![Screenshot showing icon tray widget](https://f.jstanger.dev/github/ironbar/modules/tray.png)

## Example

```corn
{
  end = [{
    type = "tray"
    direction = "top_to_bottom"
  }]
}
```

## Configuration

> Type: `tray`

%{properties}%

### Click Actions

Click actions can be one of the following built-in actions, or any custom shell command:

**Built-in actions:**
- `menu` - Opens the tray icon's popup menu
- `default` - Triggers the tray icon's default (primary) action
- `secondary` - Triggers the tray icon's secondary action
- `none` - Do nothing

**Custom commands:**

Any other string is treated as a custom shell command. Custom commands support the following placeholders:
- `{name}` - The tray item's identifier/name
- `{title}` - The tray item's title (if available)
- `{icon}` - The tray item's icon name (if available)
- `{address}` - The tray item's internal address

**Examples:**

```corn
{
  type = "tray"
  on_click_left = "menu"
  on_click_left_double = "default"
}
```

To run custom commands based on which tray item was clicked:
```corn
{
  type = "tray"
  on_click_left = "notify-send 'Clicked {name}'"
  on_click_middle = "if [ '{name}' = 'copyq' ]; then copyq toggle; fi"
}
```

## Styling

| Selector             | Description                        |
|----------------------|------------------------------------|
| `.tray`              | Tray widget box                    |
| `.tray .item`        | Tray icon button                   |
| `.tray .item.urgent` | Tray icon button (needs attention) |

For more information on styling, please see the [styling guide](styling-guide).
