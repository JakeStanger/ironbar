Displays system power information such as the battery percentage, and estimated time to empty.

> [!NOTE]
> This module requires that `upower` is installed and its service running.

`TODO: ADD SCREENSHOT`

[//]: # (![Screenshot]&#40;https://user-images.githubusercontent.com/5057870/184540521-2278bdec-9742-46f0-9ac2-58a7b6f6ea1d.png&#41;)


## Configuration

> Type: `battery`

| Name         | Type                 | Default         | Description                                                                                                                                          |
|--------------|----------------------|-----------------|------------------------------------------------------------------------------------------------------------------------------------------------------|
| `format`     | `string`             | `{percentage}%` | Format string to use for the widget button label.                                                                                                    |
| `icon_size`  | `integer`            | `24`            | Size to render icon at.                                                                                                                              |
| `thresholds` | `Map<string, float>` | `{}`            | Map of threshold names to apply as classes against the percentage at which to apply them. The nearest value above the current percentage is applied. |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "battery",
      "format": "{percentage}%",
      "thresholds": {
        "warning": 20,
        "critical": 5
      }
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "battery"
format = "{percentage}%"

[end.thresholds]
warning = 20
critical = 5
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "battery"
    format: "{percentage}%"
    thresholds:
      warning: 20
      critical: 5
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "battery"
      format = "{percentage}%"
      thresholds.warning = 20
      thresholds.critical = 5
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

| Selector                  | Description                                     |
|---------------------------|-------------------------------------------------|
| `.battery`                | Battery widget button.                          |
| `.battery.<threshold>`    | Battery widget button (dynamic threshold class) |
| `.battery .contents`      | Battery widget button contents.                 |
| `.battery .icon`          | Battery widget battery icon.                    |
| `.battery .label`         | Battery widget button label.                    |
| `.popup-battery`          | Battery popup box.                              |
| `.popup-battery .details` | Label inside the popup.                         |

For more information on styling, please see the [styling guide](styling-guide).
