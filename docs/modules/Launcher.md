> [!NOTE]
> This module requires a `wlroots-based` compositor. It will not work without the [wlr-foreign-toplevel-management](https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1) protocol.

Windows-style taskbar that displays running windows, grouped by program.
Hovering over a program with multiple windows open shows a popup with each window.
Clicking an icon/popup item focuses or launches the program.
Optionally displays a launchable set of favourites.

![Screenshot showing several open applications, including a popup showing multiple terminal windows.](https://f.jstanger.dev/github/ironbar/launcher.png)

## Configuration

> Type: `launcher`

|              | Type       | Default | Description                                                                                         |
|--------------|------------|---------|-----------------------------------------------------------------------------------------------------|
| `favorites`  | `string[]` | `[]`    | List of app IDs (or classes) to always show at the start of the launcher                            |
| `show_names` | `boolean`  | `false` | Whether to show app names on the button label. Names will still show on tooltips when set to false. |
| `show_icons` | `boolean`  | `true`  | Whether to show app icons on the button.                                                            |
| `icon_size`  | `integer`  | `32`    | Size to render icon at (image icons only).                                                          |
| `reversed`   | `boolean`  | `false` | Whether to reverse the order of favorites/items                                                     |
<details>
<summary>JSON</summary>

```json
{
  "start": [
    {
      "type": "launcher",
      "favourites": [
        "firefox",
        "discord"
      ],
      "show_names": false,
      "show_icons": true,
      "reversed": false
    }
  ]
}


```

</details>

<details>
<summary>TOML</summary>

```toml
[[start]]
type = "launcher"
favorites = ["firefox", "discord"]
show_names = false
show_icons = true
reversed = false
```

</details>

<details>
<summary>YAML</summary>

```yaml
start:
  - type: "launcher"
    favorites:
      - firefox
      - discord
    show_names: false
    show_icons: true
    reversed: false
```

</details>

<details>
<summary>Corn</summary>

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

</details>

## Styling

| Selector                      | Description              |
|-------------------------------|--------------------------|
| `.launcher`                   | Launcher widget box      |
| `.launcher .item`             | App button               |
| `.launcher .item.open`        | App button (open app)    |
| `.launcher .item.focused`     | App button (focused app) |
| `.launcher .item.urgent`      | App button (urgent app)  |
| `.popup-launcher`             | Popup container          |
| `.popup-launcher .popup-item` | Window button in popup   |

For more information on styling, please see the [styling guide](styling-guide).