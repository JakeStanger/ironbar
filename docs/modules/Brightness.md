Brightness information about screen or led brightness levels in percent.
Allows to change the respective value via scrolling.

## Configuration

> Type: `brightness`

| Name                  | Type                    | Default                | Profile? | Description                                                                                                                                                                                     |
|-----------------------|-------------------------|------------------------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `format`              | `string`                | `{percentage}%`        | Yes      | Format string to use for the widget button label.                                                                                                                                               |
| `mode.type`           | `systemd` or `keyboard` | `systemd`              | No       | The data backend of the brightness module, this can be either the KdbBrightness dbus which is good for keyboard data, or the more general login1 dbus in combination with /sys/class/<subsystem>. |
| `mode.subsystem`      | `backlight` or `leds`   | `backlight`            | No       | The name of the subsystem use on the filesystem.                                                                                                                                                |
| `mode.name`           | `string` or `null`      | `null`                 | No       | When set, using the specific directory, within /sys/class/<subsystem> . If null the module will try to find a reasonable default.                                                                |
| `smooth_scroll_speed` | `float`                 | `1.0`                  | No       | Controls how fast the brightness is changed, e.g. in case touchpad scrolling is used. Negative values swap the scroll direction.                                                                 |

Information on the profiles system can be found [here](profiles).

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "brightness",
      "format": "{percentage}%",
      "smooth_scroll_speed": 0.5,
      "mode": {
        "type": "systemd",
        "subsystem": "backlight",
        "name": "amdgpu_bl1"
      },
      "profiles": {
        "low": {
          "when": 25,
          "format": " {percentage}%"
        },
        "high": {
          "when": 100,
          "format": " {percentage}%"
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
type = "brightness"
format = "{percentage}%"
smooth_scroll_speed = 0.5

[end.mode]
type = "systemd"
subsystem = "backlight"
name = "amdgpu_bl1"

[end.profiles.low]
when = 25
format = " {percentage}%"

[end.profiles.high]
when = 100
format = " {percentage}%"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: brightness
    format: "{percentage}%"
    smooth_scroll_speed: 0.5

    mode:
      type: systemd
      subsystem: backlight
      name: amdgpu_bl1

    profiles:
      low:
        when: 25
        format: " {percentage}%"
      high:
        when: 100
        format: " {percentage}%"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "brightness"
      format = "{percentage}%"
      smooth_scroll_speed = 0.5

      mode.type = "systemd"
      mode.subsystem = "backlight"
      mode.name = "amdgpu_bl1"

      profiles.low.when = 25
      profiles.low.format = " {percentage}%"

      profiles.high.when = 100
      profiles.high.format = " {percentage}%"
    }
  ]
}
```

</details>

### Formatting Tokens

The following tokens can be used in the `format` config option:

| Token          | Description                            |
|----------------|----------------------------------------|
| `{percentage}` | The active brightness percentage.      |

## Styling

| Selector              | Description                           |
|-----------------------|---------------------------------------|
| `.brightness`         | Brightness widget button              |
| `.brightness .label`  | Notifications widget button           |

For more information on styling, please see the [styling guide](styling-guide).
