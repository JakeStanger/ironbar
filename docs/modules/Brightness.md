Brightness information about screen or led brightness levels in percent.
Allows to change the respective value via scrolling.

## Configuration

> Type: `brightness`

| Name                   | Type                    | Default         | Profile? | Description                                                                                                                                                                                       |
|------------------------|-------------------------|-----------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `format`               | `string`                | `{percentage}%` | Yes      | Format string to use for the widget button label.                                                                                                                                                 |
| `icon_label`           | `string`                | `null`          | Yes      | Icon to show alongside the label. Supports [image](images) icons.                                                                                                                                 |
| `mode.type`            | `systemd` or `keyboard` | `systemd`       | No       | The data backend of the brightness module, this can be either the KdbBrightness dbus which is good for keyboard data, or the more general login1 dbus in combination with /sys/class/{subsystem}. |
| `mode.subsystem`       | `backlight` or `leds`   | `backlight`     | No       | The name of the subsystem use on the filesystem.                                                                                                                                                  |
| `mode.name`            | `string` or `null`      | `null`          | No       | When set, using the specific directory, within /sys/class/{subsystem} . If null the module will try to find a reasonable default.                                                                 |
| `smooth_scroll_speed`  | `float`                 | `1.0`           | No       | Controls how fast the brightness is changed, e.g. in case touchpad scrolling is used. Negative values swap the scroll direction.                                                                  |
| `use_default_profiles` | `boolean`               | `true`          | No       | Whether default profiles should be used.                                                                                                                                                          |

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
          "format": "{percentage}%",
          "icon_label": ""
        },
        "high": {
          "when": 100,
          "format": "{percentage}%",
          "icon_label": ""
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
format = "{percentage}%"
icon_label = ""

[end.profiles.high]
when = 100
format = "{percentage}%"
icon_label = ""
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
        format: "{percentage}%"
        icon_label: ""
      high:
        when: 100
        format: "{percentage}%"
        icon_label: ""
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
      profiles.low.format = "{percentage}%"
      profiles.low.icon_label = ""

      profiles.high.when = 100
      profiles.high.format = "{percentage}%"
      profiles.high.icon_label = ""
    }
  ]
}
```

</details>

### Icons

The icon is configured per profile using the `icon_label` option.

Brightness ships with default profiles that set icons for percentage ranges:

| Profile    | Threshold (<=) | Icon |
|------------|----------------|------|
| `level0`   | `5`            | ``  |
| `level10`  | `15`           | ``  |
| `level20`  | `25`           | ``  |
| `level30`  | `35`           | ``  |
| `level40`  | `45`           | ``  |
| `level50`  | `55`           | ``  |
| `level60`  | `65`           | ``  |
| `level70`  | `75`           | ``  |
| `level80`  | `85`           | ``  |
| `level90`  | `95`           | ``  |
| `level100` | `100`          | ``  |

### Default profiles

<details>
<summary>Show</summary>

```corn
{
    level0.when = 5.0
    level0.icon_label = ""

    level10.when = 15.0
    level10.icon_label = ""
    
    level20.when = 25.0
    level20.icon_label = ""
    
    level30.when = 35.0
    level30.icon_label = ""
    
    level40.when = 45.0
    level40.icon_label = ""
    
    level50.when = 55.0
    level50.icon_label = ""
    
    level60.when = 65.0
    level60.icon_label = ""
    
    level70.when = 75.0
    level70.icon_label = ""
    
    level80.when = 85.0
    level80.icon_label = ""

    level90.when = 95.0
    level90.icon_label = ""
    
    level100.when = 100.0
    level100.icon_label = ""
}
```

</details>

### Formatting Tokens

The following tokens can be used in the `format` config option:

| Token          | Description                            |
|----------------|----------------------------------------|
| `{percentage}` | The active brightness percentage.      |

## Styling

| Selector              | Description                                |
|-----------------------|--------------------------------------------|
| `.brightness`         | Brightness widget button                   |
| `.brightness .label`  | text, which is controlled via `format`     |
| `.brightness .icon`   | icon, which is controlled via `icon_label` |

For more information on styling, please see the [styling guide](styling-guide).
