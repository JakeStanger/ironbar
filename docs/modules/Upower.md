Displays system power information such as the battery percentage, and estimated time to empty.

`TODO: ADD SCREENSHOT`

[//]: # (![Screenshot]&#40;https://user-images.githubusercontent.com/5057870/184540521-2278bdec-9742-46f0-9ac2-58a7b6f6ea1d.png&#41;)


## Configuration

> Type: `upower`

| Name        | Type      | Default         | Description                                       |
|-------------|-----------|-----------------|---------------------------------------------------|
| `format`    | `string`  | `{percentage}%` | Format string to use for the widget button label. |
| `icon_size` | `integer` | `24`            | Size to render icon at.                           |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "upower",
      "format": "{percentage}%"
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "upower"
format = "{percentage}%"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "upower"
    format: "{percentage}%"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "upower"
      format = "{percentage}%"
    }
  ]
}
```

</details>

## Styling

| Selector                        | Description                    |
|---------------------------------|--------------------------------|
| `.upower`                       | Upower widget container.       |
| `.upower .button`               | Upower widget button.          |
| `.upower .button .contents`     | Upower widget button contents. |
| `.upower .button .icon`         | Upower widget battery icon.    |
| `.upower .button .label`        | Upower widget button label.    |
| `.popup-upower`                 | Upower popup box.              |
| `.popup-upower .upower-details` | Label inside the popup.        |

For more information on styling, please see the [styling guide](styling-guide).
