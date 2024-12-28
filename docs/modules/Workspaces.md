> ⚠ **This module is currently only supported on Sway and Hyprland**

Shows all current workspaces. Clicking a workspace changes focus to it.

![Screenshot showing workspaces widget using custom icons with browser workspace focused](https://user-images.githubusercontent.com/5057870/184540156-26cfe4ec-ab8d-4e0f-a883-8b641025366b.png)

## Configuration

> Type: `workspaces`

| Name           | Type                                  | Default | Description                                                                                                                                                               |
|----------------|---------------------------------------|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `name_map`     | `Map<string, string or image>`        | `{}`    | A map of actual workspace names to their display labels/images. Workspaces use their actual name if not present in the map. See [here](images) for information on images. |
| `favorites`    | `Map<string, string[]>` or `string[]` | `[]`    | Workspaces to always show. This can be for all monitors, or a map to set per monitor.                                                                                     |
| `hidden`       | `string[]`                            | `[]`    | A list of workspace names to never show                                                                                                                                   |
| `icon_size`    | `integer`                             | `32`    | Size to render icon at (image icons only).                                                                                                                                |
| `all_monitors` | `boolean`                             | `false` | Whether to display workspaces from all monitors. When `false`, only shows workspaces on the current monitor.                                                              |
| `sort`         | `'added'` or `'label'` or `'name'`    | `label` | The method used for sorting workspaces. `added` always appends to the end, `label` sorts by displayed value, and `name` sorts by workspace name.                          |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "workspaces",
      "name_map": {
        "1": "",
        "2": "",
        "3": ""
      },
      "favorites": ["1", "2", "3"],
      "all_monitors": false
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "workspaces"
all_monitors = false
favorites = ["1", "2", "3"]

[end.name_map]
1 = ""
2 = ""
3 = ""

```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "workspaces"
    name_map:
      1: ""
      2: ""
      3: ""
    favorites:
      - "1"
      - "2"
      - "3"
    all_monitors: false
```

</details>

<details>
<summary>Corn</summary>

```corn
{
    end = [
        {
            type = "workspaces",
            name_map.1 = ""
            name_map.2 = ""
            name_map.3 = ""
            favorites = [ "1" "2" "3" ]
            all_monitors = false
        }
    ]
}
```

</details>

## Styling

| Selector                       | Description                                             |
| ------------------------------ | ------------------------------------------------------- |
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
