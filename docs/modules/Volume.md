> [!NOTE]
> This requires PulseAudio to function (`pipewire-pulse` is supported).

Displays the current volume level.
Clicking on the widget opens a volume mixer, which allows you to change the device output level,
the default playback device, and control application volume levels individually.
Use `truncate` or `marquee` options to control the display of application titles in the volume mixer.

![The volume widget, with its popup open. A single stream is playing audio.](https://f.jstanger.dev/github/ironbar/modules/volume.png)

## Jargon

The volume module uses [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) under the hood and therefore
inherits some of its termonology to define its behavior and implementation. Here are a few common terms related that
pulseaudio uses to describe sources of audio:

- `Sink` = a sound device producing audio coming out of your machine (speakers)
- `Source` = sound device receiving audio going into your machine (microphone)
- `SinkInput` = an application/program sending sound to an existing sink (app using speakers)
- `SourceOutput` = an application/program receiving audio from a source (app using microphone)

> [!NOTE]
> These names are not fixed, and are mostly used because I have failed to find anything better.
> If you have any suggestions, please submit them!

## Example

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

## Configuration

> Type: `volume`

%{properties}%

## Styling

| Selector                                      | Description                                                             |
|-----------------------------------------------|-------------------------------------------------------------------------|
| `.volume`                                     | Volume widget button.                                                   |
| `.volume .sink`                               | Volume widget sink (speaker) label.                                     |
| `.volume .source`                             | Volume widget source (microphone) label.                                |
| `.popup-volume`                               | Volume popup box.                                                       |
| `.popup-volume .device-box`                   | Box for the device volume controls.                                     |
| `.popup-volume .device-box .sink-box`         | Box for the sink volume controls.                                       |
| `.popup-volume .device-box .source-box`       | Box for the source volume controls.                                     |
| `.popup-volume .device-box .device-selector`  | Default device dropdown selector.                                       |
| `.popup-volume .device-box .slider`           | Device volume slider.                                                   |
| `.popup-volume .device-box .btn-mute`         | Device volume mute toggle button.                                       |
| `.popup-volume .apps-box`                     | Parent box for the application volume controls.                         |
| `.popup-volume .apps-box .sink-input-box`     | Parent box for the application volume controls (sink inputs).           |
| `.popup-volume .apps-box .source-output-box`  | Parent box for the application volume controls (source outputs).        |
| `.popup-volume .apps-box .app-box`            | Box for an individual application volume controls (all).                |
| `.popup-volume .apps-box .app-box.input-box`  | Box for an individual application volume controls (sink input only).    |
| `.popup-volume .apps-box .app-box.output-box` | Box for an individual application volume controls (source output only). |
| `.popup-volume .apps-box .app-box .title`     | Name of the application playback stream.                                |
| `.popup-volume .apps-box .app-box .slider`    | Application volume slider.                                              |
| `.popup-volume .apps-box .app-box .btn-mute`  | Application volume mute toggle button.                                  |

For more information on styling, please see the [styling guide](styling-guide).
