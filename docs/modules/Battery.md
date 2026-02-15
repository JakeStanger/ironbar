Displays system power information such as the battery percentage, and estimated time to empty.

> [!NOTE]
> This module requires that `upower` is installed and its service running.

`TODO: ADD SCREENSHOT`

[//]: # (![Screenshot]&#40;https://user-images.githubusercontent.com/5057870/184540521-2278bdec-9742-46f0-9ac2-58a7b6f6ea1d.png&#41;)


## Configuration

> Type: `battery`

| Name         | Type      | Default         | Profile? | Description                                       |
|--------------|-----------|-----------------|----------|---------------------------------------------------|
| `format`     | `string`  | `{percentage}%` | Yes      | Format string to use for the widget button label. |
| `icon_size`  | `integer` | `24`            | No       | Size to render icon at.                           |
| `show_icon`  | `boolean` | `true`          | No       | Whether to show the icon.                         |
| `show_label` | `boolean` | `true`          | No       | Whether to show the label.                        |

This module uses **a compound threshold** with 1-2 values for profiles:

- `percent` - `0-100`
- `charging` - `true`/`false` (optional)

Information on the profiles system can be found [here](profiles).

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "battery",
      "format": "{percentage}%",
      "profiles": {
        "warning": 20,
        "critical": {
          "when": { "percent":  5, "charging": false },
          "format": "[LOW] {percentage}%"
        }
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

[end.profiles]
warning = 20

[end.profiles.critical]
when = { percent = 5, charging = false }
format = "[LOW] {percentage}%"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "battery"
    format: "{percentage}%"
    profiles:
      warning: 20
      critical:
        when:
          percent: 5
          charging: false
        format: "[LOW] {percentage}%"
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
      profiles.warning = 20
      
      profiles.critical.when = { percent = 5 charging = false }
      profiles.critical.format = "[LOW] {percentage}%"
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
