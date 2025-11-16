Brightness information about screen or led brighness levels in percent.
Allows to change the respective value via scrolling.

## Configuration

> Type: `brightness`

| Name                            | Type                                 | Default                                                                                                | Description                                                                     |
|---------------------------------|--------------------------------------|--------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| `format`                        | `string`                             | `{icon} {percentage}%`                                                                                 | Format string to use for the widget button label.                               |
| `icons.brighness`               | `(integer, string)[]`                | `[(0, ""), (12, ""), (24, ""), (36, ""), (48, ""), (60,""), (72, ""), (84, ""), (100, "")]` | Icons to show, based on the respective brightness level. Needs to be sorted     |
| `datasource`                    | Map `Login1Fs` or string `Keyboard`  | `Login1Fs`                                                                                             | The data backend of the brightness module, this can be either the KdbBrightness dbus which is good for keyboard data, or the more general Login1 dbus in combination with /sys/class/<subsystem> filesystem |
| `datasource.Login1Fs.subsystem` | `backlight` or `leds`                | `backlight`                                                                                            | The name of the subsystem use on the filesystem                                 |
| `datasource.Login1Fs.name`      | `string` or `null`                   | `null`                                                                                                 | When set, using the specific directory, within /sys/class/<subsystem> . If null the module will try to find a reasonable default          |
| `interval`                      | `integer`                            | `1000`                                                                                                 | Polling interval for getting brightness value in `ms`                           |
| `smooth_scroll_speed`           | `float`                              | `1.0`                                                                                                  | Allows to controll how fast the brightness is changed, e.g. in case touchpad scrolling is used. Negative values swap the scroll direction |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "brightness",
      "format": "{icon} {percentage}%",
      "interval": 200,
      "smooth_scroll_speed": 0.5,
      "datasource": {
        "Login1Fs": {
          "subsystem": "backlight",
          "name": "amdgpu_bl1"
        }
      },
      "icons": {
        "brightness": [
          [0, ""],
          [50, ""],
          [100, ""]
        ]
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
format = "{icon} {percentage}%"
interval = 200
smooth_scroll_speed = 0.5

[end.datasource.Login1Fs]
subsystem = "backlight"
name = "amdgpu_bl1"

[end.icons]
brightness = [
    [0,   ""],
    [50,  ""],
    [100, ""]
]
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: brightness
    format: "{icon} {percentage}%"
    interval: 200
    smooth_scroll_speed: 0.5

    datasource:
      Login1Fs:
        subsystem: backlight
        name: amdgpu_bl1

    icons:
      brightness:
        - [0, ""]
        - [50, ""]
        - [100, ""]
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "brightness"
      format = "{icon} {percentage}%"
      interval = 200
      smooth_scroll_speed = 0.5

      datasource.Login1Fs.subsystem = "backlight"
      datasource.Login1Fs.name = "amdgpu_bl1"

      icons.brightness = [ [ 0 "" ] [ 50 " ] [ 100 "" ] ]
    }
  ]
}
```

</details>

## Styling

For more information on styling, please see the [styling guide](styling-guide).
