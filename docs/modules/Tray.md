Displays a fully interactive icon tray using the KDE `libappindicator` protocol.

![Screenshot showing icon tray widget](https://user-images.githubusercontent.com/5057870/184540135-78ffd79d-f802-4c79-b09a-05a733dadc55.png)

## Configuration

> Type: `tray`

| Name                 | Type      | Default                                                         | Description                                                                                                                                                         |
|----------------------|-----------|-----------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `direction`          | `string`  | `left_to_right` if bar is horizontal, `top_to_bottom` otherwise | Direction to display the tray items. Possible values: `top_to_bottom`, `bottom_to_top`, `left_to_right`, `right_to_left`                                            |
| `icon_size`          | `integer` | `16`                                                            | Size in pixels to display tray icons as.                                                                                                                            |
| `prefer_theme_icons` | `bool`    | `true`                                                          | Requests that icons from the theme be used over the item-provided item. Most items only provide one or the other so this will have no effect in most circumstances. |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "tray",
      "direction": "top_to_bottom"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "tray"
direction = "top_to_bottom"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "tray"
    direction: "top_to_bottom"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [{
    type = "tray"
    direction = "top_to_bottom"
  }]
}
```

</details>

## Styling

| Selector      | Description      |
|---------------|------------------|
| `.tray`       | Tray widget box  |
| `.tray .item` | Tray icon button |

For more information on styling, please see the [styling guide](styling-guide).
