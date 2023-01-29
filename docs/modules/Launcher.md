Windows-style taskbar that displays running windows, grouped by program.
Hovering over a program with multiple windows open shows a popup with each window.
Clicking an icon/popup item focuses or launches the program.
Optionally displays a launchable set of favourites.

![Screenshot showing several open applications, including a focused terminal.](https://user-images.githubusercontent.com/5057870/184540058-120e190e-2a45-4167-99c7-ed76482d1f16.png)

## Configuration

> Type: `launcher`

|              | Type       | Default | Description                                                                                         |
|--------------|------------|---------|-----------------------------------------------------------------------------------------------------|
| `favorites`  | `string[]` | `[]`    | List of app IDs (or classes) to always show at the start of the launcher                            |
| `show_names` | `boolean`  | `false` | Whether to show app names on the button label. Names will still show on tooltips when set to false. |
| `show_icons` | `boolean`  | `true`  | Whether to show app icons on the button.                                                            |

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
      "icon_theme": "Paper"
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
icon_theme = "Paper"
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
    icon_theme: "Paper"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  start = [
    {
      type = "launcher"
      favorites = ["firefox" "discord"]
      show_names = false
      show_icons = true
      icon_theme = "Paper"

    }
  ]
}
```

</details>

## Styling

| Selector                      | Description              |
|-------------------------------|--------------------------|
| `#launcher`                   | Launcher widget box      |
| `#launcher .item`             | App button               |
| `#launcher .item.open`        | App button (open app)    |
| `#launcher .item.focused`     | App button (focused app) |
| `#launcher .item.urgent`      | App button (urgent app)  |
| `#launcher-popup`             | Popup container          |
| `#launcher-popup .popup-item` | Window button in popup   |
