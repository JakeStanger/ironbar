> [!NOTE]
> This module requires the [wlr-foreign-toplevel-management](https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1) protocol.

Displays the title and/or icon of the currently focused window.

![Screenshot of focused widget, showing an Ironbar file currently open in RustRover](https://f.jstanger.dev/github/ironbar/modules/focused.png)

## Example

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

## Configuration

> Type: `focused`

%{properties}%

## Styling

| Selector          | Description        |
|-------------------|--------------------|
| `.focused`        | Focused widget box |
| `.focused .icon`  | App icon           |
| `.focused .label` | App name           |

For more information on styling, please see the [styling guide](styling-guide).
