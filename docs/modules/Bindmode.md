> [!NOTE]
> This module is currently only available on Sway and Hyprland.

Displays Sway's current binding mode or [Hyprland's current submap](https://wiki.hyprland.org/Configuring/Binds/#submaps)
in a label. Nothing is displayed if no binding mode is active.

## Example

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

## Configuration

> Type: `bindmode`

%{properties}%

## Styling

| Selector    | Description            |
|-------------|------------------------|
| `.bindmode` | Bind mode label widget |

For more information on styling, please see the [styling guide](styling-guide).
