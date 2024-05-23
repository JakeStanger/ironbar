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

### Formatting Tokens

The following tokens can be used in the `format` config option,
and will be replaced with values from the current battery state:

| Token               | Description                              |
|---------------------|------------------------------------------|
| `{percentage}`      | The battery charge percentage.           |
| `{state}`           | The current battery (dis)charging state. |
| `{time_remaining}`  | The ETA to battery empty or full.        |

## Styling

| Selector                        | Description                    |
|---------------------------------|--------------------------------|
| `.upower`                       | Upower widget button.          |
| `.upower .contents`             | Upower widget button contents. |
| `.upower .icon`                 | Upower widget battery icon.    |
| `.upower .label`                | Upower widget button label.    |
| `.popup-upower`                 | Upower popup box.              |
| `.popup-upower .upower-details` | Label inside the popup.        |

For more information on styling, please see the [styling guide](styling-guide).
