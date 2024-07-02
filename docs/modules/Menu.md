Application menu that shows installed programs and optionally custom entries. Clicking the menu button will open the main menu, clicking on any application category will open a sub-menu with any installed applications that match.

## Configuration

> Type: `menu`

|              | Type       | Default | Description                                                                                         |
|--------------|------------|---------|-----------------------------------------------------------------------------------------------------|
| `start`  | `MenuEntry[]` | `[]`    | List of menu entries |
| `center` | `MenuEntry[]`  | default XDG menu | List of menu entries. By default this shows a number of XDG entries that should cover all common applications |
| `end` | `MenuEntry[]` | `[]`    | List of menu entries |
| `height`  | `integer | null`  | `null`    | The height of the menu, leave null for it to resize dynamically |
| `width`   | `integer | null`  | `null` | The width of the menu, leave null for it to resize dynamically |
| `max_label_length`   | `integer`  | `25` | Maximum length for the label of an XDG entry |
| `label`   | `string | null`  | `≡` | The label of the button that opens the menu |
| `label_icon`   | `string | null`  | `null` | An icon (from icon theme) to display on the button which opens the application menu |
| `label_icon_size`   | `integer`  | `16` | Size of the label_icon if one is supplied |


> Type: `MenuEntry`

|              | Type       | Default | Description                                                                                         |
|--------------|------------|---------|-----------------------------------------------------------------------------------------------------|
| `type`  | `xdg_entry | xdg_other | custom` |    | Type of the entry |
| `label` | `string`  | | Label of the entry's button |
| `icon` | `string | null` | `null` | Icon for the entry's button |
| `categories`  | `string[]` | | If `xdg_entry` this is is the list of freedesktop.org categories to include in this entry's sub menu |
| `on_click`   | `string`  | | If `custom` this is a shell command to execute when the entry's button is clicked |

<details>

<summary>JSON</summary>

```json
{
  "start": [
    {
      "type": "menu",
      "start": [
        {
            "type": "custom",
            "label": "Terminal",
            "on_click": "xterm",
        }
      ],
      "height": 440,
      "width": 200,
      "icon": "archlinux",
      "label": null
    }
  ]
}


```

</details>

<details>
<summary>TOML</summary>

```toml
[[start.menu]]
height = 400
width = 200
icon = "archlinux"
label = null

[[start.menu.start]]
type = "custom"
label = "Terminal"
on_click = "xterm"
```

</details>

<details>
<summary>YAML</summary>

```yaml
start:
  - type: "menu"
    start:
      - type: custom
        label: Terminal
        on_click: xterm
    height: 440
    width: 200
    icon: archlinux
    label: null
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  start = [
    {
      type = "menu"
      start = [
        {
            type = "custom"
            label = "Terminal"
            on_click = "xterm"
        }
      ]
      height = 440
      width = 200
      icon = "archlinux"
      label = null
    }
  ]
}
```

</details>

## Styling

| Selector                      | Description                    |
|-------------------------------|--------------------------------|
| `.menu`                       | Menu button                    |
| `.menu-popup`                 | Main container of the popup    |
| `.menu-popup_main`            | Main menu of the menu          |
| `.menu-popup_main_start`      | Container for `start` entries  |
| `.menu-popup_main_center`     | Container for `center` entries |
| `.menu-popup_main_end`        | Container for `end` entries    |
| `.menu-popup_sub-menu`        | All sub-menues                 |

For more information on styling, please see the [styling guide](styling-guide).