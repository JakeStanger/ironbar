> [!NOTE]
> - This module requires your user is in the `input` group.
> - The keyboard layout feature is only available on Sway and Hyprland.

Displays the toggle state of the capslock, num lock and scroll lock keys, and the current keyboard layout.

![Screenshot of keyboard widget](https://f.jstanger.dev/github/ironbar/keys.png)

## Example

```corn
{
end = [ 
        { 
            type = "keyboard" 
            show_scroll = false 
            icons.caps_on = "󰪛" 
            icons.layout_map.'English (US)' = "🇺🇸"
            icons.layout_map.Ukrainian = "🇺🇦"
        }
    ]
}
```

## Configuration

> Type: `keyboard`

%{properties}%

## Styling

| Selector                   | Description                                |
|----------------------------|--------------------------------------------|
| `.keyboard`                | Keys box container widget.                 |
| `.keyboard .key`           | Individual key indicator container widget. |
| `.keyboard .key.enabled`   | Key indicator where key is toggled on.     |
| `.keyboard .key.caps`      | Capslock key indicator.                    |
| `.keyboard .key.num`       | Num lock key indicator.                    |
| `.keyboard .key.scroll`    | Scroll lock key indicator.                 |
| `.keyboard .key.image`     | Key indicator image icon.                  |
| `.keyboard .key.text-icon` | Key indicator textual icon.                |
| `.keyboard .layout`        | Keyboard layout indicator.                 |

For more information on styling, please see the [styling guide](styling-guide).
