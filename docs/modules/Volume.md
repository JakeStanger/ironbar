Displays the current volume level.
Clicking on the widget opens a volume mixer, which allows you to change the device output level,
the default playback device, and control application volume levels individually.
Use `truncate` or `marquee` options to control the display of application titles in the volume mixer.

This requires PulseAudio to function (`pipewire-pulse` is supported).

![The volume widget, with its popup open. A single stream is playing audio.](https://f.jstanger.dev/github/ironbar/modules/volume.png)

## Configuration

> Type: `volume`

| Name                            | Type                                                 | Default                | Description                                                                                                                       |
|---------------------------------|------------------------------------------------------|------------------------|-----------------------------------------------------------------------------------------------------------------------------------|
| `format`                        | `string`                                             | `{icon} {percentage}%` | Format string to use for the widget button label.                                                                                 |
| `sink_slider_orientation`       | `'vertical'` or `'horizontal'`                       | `vertical`             | The orientation of the sink slider.                                                                                               |
| `max_volume`                    | `float`                                              | `100`                  | Maximum value to allow volume sliders to reach. Pulse supports values > 100 but this may result in distortion.                    |
| `icons.volume_high`             | `string`                                             | `󰕾`                    | Icon to show for high volume levels.                                                                                              |
| `icons.volume_medium`           | `string`                                             | `󰖀`                    | Icon to show for medium volume levels.                                                                                            |
| `icons.volume_low`              | `string`                                             | `󰕿`                    | Icon to show for low volume levels.                                                                                               |
| `icons.muted`                   | `string`                                             | `󰝟`                    | Icon to show for muted outputs.                                                                                                   |
| `truncate`                      | `'start'` or `'middle'` or `'end'` or `off` or `Map` | `off`                  | The location of the ellipses and where to truncate text from. Leave null to avoid truncating. Use the long-hand `Map` version if specifying a length. Takes precedence over `marquee` if both are configured. |
| `truncate.mode`                 | `'start'` or `'middle'` or `'end'` or `off`          | `off`                  | The location of the ellipses and where to truncate text from. Leave null to avoid truncating.                                     |
| `truncate.length`               | `integer`                                            | `null`                 | The fixed width (in chars) of the widget. Leave blank to let GTK automatically handle.                                            |
| `truncate.max_length`           | `integer`                                            | `null`                 | The maximum number of characters before truncating. Leave blank to let GTK automatically handle.                                  |
| `marquee`                       | `Map`                                                | `false`                | Options for enabling and configuring a marquee (scrolling) effect for long text. Ignored if `truncate` is configured.             |
| `marquee.enable`                | `bool`                                               | `false`                | Whether to enable a marquee effect.                                                                                               |
| `marquee.max_length`            | `integer`                                            | `null`                 | The maximum length of text (roughly, in characters) before it gets truncated and starts scrolling.                                |
| `marquee.scroll_speed`          | `float`                                              | `0.5`                  | Scroll speed in pixels per frame. Higher values scroll faster.                                                                    |
| `marquee.pause_duration`        | `integer`                                            | `5000`                 | Duration in milliseconds to pause at each loop point.                                                                             |
| `marquee.separator`             | `string`                                             | `"    "`               | String displayed between the end and beginning of text as it loops.                                                               |
| `marquee.on_hover`              | `'none'` or `'pause'` or `'play'`                    | `'none'`               | Controls marquee behavior on hover: `'none'` (always scroll), `'pause'` (pause on hover), or `'play'` (only scroll on hover).     |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "volume",
      "format": "{icon} {percentage}%",
      "sink_slider_orientation": "vertical",
      "max_volume": 100,
      "truncate": "middle",
      "icons": {
        "volume_high": "󰕾",
        "volume_medium": "󰖀",
        "volume_low": "󰕿",
        "muted": "󰝟"
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
type = "volume"
format = "{icon} {percentage}%"
sink_slider_orientation = "vertical",
max_volume = 100
truncate = "middle"

[end.icons]
volume_high = "󰕾"
volume_medium = "󰖀"
volume_low = "󰕿"
muted = "󰝟"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "volume"
    format: "{icon} {percentage}%"
    sink_slider_orientation: "vertical"
    max_volume: 100
    truncate: "middle"
    icons:
      volume_high: "󰕾"
      volume_medium: "󰖀"
      volume_low: "󰕿"
      muted: "󰝟"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "volume"
      format = "{icon} {percentage}%"
      sink_slider_orientation = "vertical"
      max_volume = 100
      truncate = "end"
      icons.volume_high = "󰕾"
      icons.volume_medium = "󰖀"
      icons.volume_low = "󰕿"
      icons.muted = "󰝟"
    }
  ]
}
```

</details>

### Formatting Tokens

The following tokens can be used in the `format` config option:

| Token          | Description                               |
|----------------|-------------------------------------------|
| `{percentage}` | The active device volume percentage.      |
| `{icon}`       | The icon representing the current volume. |
| `{name}`       | The active device name.                   |

## Styling

| Selector                                     | Description                                        |
|----------------------------------------------|----------------------------------------------------|
| `.volume`                                    | Volume widget button.                              |
| `.popup-volume`                              | Volume popup box.                                  |
| `.popup-volume .device-box`                  | Box for the device volume controls.                |
| `.popup-volume .device-box .device-selector` | Default device dropdown selector.                  |
| `.popup-volume .device-box .slider`          | Device volume slider.                              |
| `.popup-volume .device-box .btn-mute`        | Device volume mute toggle button.                  |
| `.popup-volume .apps-box`                    | Parent box for the application volume controls.    |
| `.popup-volume .apps-box .app-box`           | Box for an individual application volume controls. |
| `.popup-volume .apps-box .app-box .title`    | Name of the application playback stream.           |
| `.popup-volume .apps-box .app-box .slider`   | Application volume slider.                         |
| `.popup-volume .apps-box .app-box .btn-mute` | Application volume mute toggle button.             |

For more information on styling, please see the [styling guide](styling-guide).
