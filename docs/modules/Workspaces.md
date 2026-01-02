> [!NOTE]
> This module is currently only supported on Sway, Hyprland and Niri.

Shows all current workspaces. Clicking a workspace changes focus to it.

![Screenshot showing workspaces widget using custom icons with browser workspace focused](https://user-images.githubusercontent.com/5057870/184540156-26cfe4ec-ab8d-4e0f-a883-8b641025366b.png)

![Screenshot showing workspaces widget using default names with workspace 4 focused](https://f.jstanger.dev/github/ironbar/modules/workspaces.png)

## Example

```corn
{
    end = [
        {
            type = "workspaces"
            name_map.1 = ""
            name_map.2 = ""
            name_map.3 = ""
            favorites = [ "1" "2" "3" ]
            all_monitors = false
        }
    ]
}
```

## Configuration

> Type: `workspaces`

%{properties}%

## Styling

| Selector                       | Description                                             |
|--------------------------------|---------------------------------------------------------|
| `.workspaces`                  | Workspaces widget box                                   |
| `.workspaces .item`            | Workspace button                                        |
| `.workspaces .item.focused`    | Workspace button (workspace focused)                    |
| `.workspaces .item.visible`    | Workspace button (workspace visible, including focused) |
| `.workspaces .item.urgent`     | Workspace button (workspace contains urgent window)     |
| `.workspaces .item.inactive`   | Workspace button (favourite, not currently open)        |
| `.workspaces .item .icon`      | Workspace button icon (any type)                        |
| `.workspaces .item .text-icon` | Workspace button icon (textual only)                    |
| `.workspaces .item .image`     | Workspace button icon (image only)                      |

For more information on styling, please see the [styling guide](styling-guide).
