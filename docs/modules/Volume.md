> [!NOTE]
> This requires PulseAudio to function (`pipewire-pulse` is supported).

Displays the current volume level.
Clicking on the widget opens a volume mixer, which allows you to change the device output level,
the default playback device, and control application volume levels individually.
Use `truncate` or `marquee` options to control the display of application titles in the volume mixer.

![The volume widget, with its popup open. A single stream is playing audio.](https://f.jstanger.dev/github/ironbar/modules/volume.png)

## Example

```corn
{
  end = [
    {
      type = "volume"
      format = "{icon} {percentage}%"
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

## Configuration

> Type: `volume`

%{properties}%

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
