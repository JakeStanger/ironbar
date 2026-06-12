> [!NOTE]
> This module requires the [wlr-foreign-toplevel-management](https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1) protocol.

Windows-style taskbar that displays running windows, grouped by program.
Hovering over a program with multiple windows open shows a popup with each window.
Left clicking an icon/popup item focuses the program if it has any open instances or otherwise launches a new instance of the program.
Middle clicking an icon always launches a new instance of the program.
Optionally displays a launchable set of favourites.

![Screenshot showing several open applications, including a popup showing Ironbar open in Rustrover.](https://f.jstanger.dev/github/ironbar/modules/launcher.png)

> [!TIP]
> Window previews are [experimental](https://github.com/JakeStanger/ironbar/pull/1189) and have not been merged yet!

## Example

```corn
{
  start = [
    {
      type = "launcher"
      favorites = [ "firefox" "discord" ]
      show_names = false
      show_icons = true
      reversed = false
    }
  ]
}
```

## Configuration

> Type: `launcher`

%{properties}%

## Styling

| Selector                             | Description               |
|--------------------------------------|---------------------------|
| `.launcher`                          | Launcher widget box       |
| `.launcher .item`                    | App button                |
| `.launcher .item.open`               | App button (open app)     |
| `.launcher .item.focused`            | App button (focused app)  |
| `.launcher .item.urgent`             | App button (urgent app)   |
| `.launcher .pagination`              | Pagination controls box   |
| `.launcher .pagination .btn-back`    | Pagination back button    |
| `.launcher .pagination .btn-forward` | Pagination forward button |
| `.popup-launcher`                    | Popup container           |
| `.popup-launcher .popup-item`        | Window button in popup    |

For more information on styling, please see the [styling guide](styling-guide).
