Displays the title and/or icon of the currently focused window.

![Screenshot of focused widget, showing this page open on firefox](https://user-images.githubusercontent.com/5057870/184714118-c1fb1c67-cd8c-4cc0-b5cd-6faccff818ac.png)


## Configuration

> Type: `focused`

| Name                          | Type                         | Default | Description                                                                                                                                     |
|-------------------------------|------------------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `show_icon`                   | `boolean`                    | `true`  | Whether to show the app's icon                                                                                                                  |
| `show_title`                  | `boolean`                    | `true`  | Whether to show the app's title                                                                                                                 |
| `icon_size`                   | `integer`                    | `32`    | Size of icon in pixels                                                                                                                          |
| `icon_theme`                  | `string`                     | `null`  | GTK icon theme to use                                                                                                                           |
| `truncate` or `truncate.mode` | `start` or `middle` or `end` | `null`  | The location of the ellipses and where to truncate text from. Leave null to avoid truncating. Use the long-hand version if specifying a length. |
| `truncate.length`             | `integer`                    | `null`  | The maximum number of characters before truncating. Leave blank to let GTK automatically handle.                                                |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "focused",
      "show_icon": true,
      "show_title": true,
      "icon_size": 32,
      "icon_theme": "Paper"
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "focused"
show_icon = true
show_title = true
icon_size = 32
icon_theme = "Paper"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "focused"
    show_icon: true
    show_title: true
    icon_size: 32
    icon_theme: "Paper"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "focused"
      show_icon = true
      show_title = true
      icon_size = 32
      icon_theme = "Paper"
    }
  ]
}
```

</details>

## Styling

| Selector                 | Description        |
|--------------------------|--------------------|
| `#focused`               | Focused widget box |
| `#focused #icon`         | App icon           |
| `#focused #label`        | App name           |