Displays currently playing song from your music player.
This module supports both MPRIS players and MPD servers.
Clicking on the widget opens a popout displaying info about the current song, album art
and playback controls.

in MPRIS mode, the widget will listen to all players and automatically detect/display the active one.

![Screenshot showing MPD widget with track playing with popout open](https://f.jstanger.dev/github/ironbar/music.png)

## Configuration

> Type: `music`

|                       | Type                                  | Default              | Description                                                                                                                                           |
|-----------------------|---------------------------------------|----------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `player_type`         | `mpris` or `mpd`                      | `mpris`              | Whether to connect to MPRIS players or an MPD server.                                                                                                 |
| `format`              | `string`                              | `{title} / {artist}` | Format string for the widget. More info below.                                                                                                        |
| `truncate`            | `start` or `middle` or `end` or `Map` | `null`               | The location of the ellipses and where to truncate text from. Leave null to avoid truncating. Use the long-hand `Map` version if specifying a length. |
| `truncate.mode`       | `start` or `middle` or `end`          | `null`               | The location of the ellipses and where to truncate text from. Leave null to avoid truncating.                                                         |
| `truncate.length`     | `integer`                             | `null`               | The fixed width (in chars) of the widget. Leave blank to let GTK automatically handle.                                                                |
| `truncate.max_length` | `integer`                             | `null`               | The maximum number of characters before truncating. Leave blank to let GTK automatically handle.                                                      |
| `icons.play`          | `string/image`                        | ``                  | Icon to show when playing.                                                                                                                            |
| `icons.pause`         | `string/image`                        | ``                  | Icon to show when paused.                                                                                                                             |
| `icons.prev`          | `string/image`                        | `玲`                  | Icon to show on previous button.                                                                                                                      |
| `icons.next`          | `string/image`                        | `怜`                  | Icon to show on next button.                                                                                                                          |
| `icons.volume`        | `string/image`                        | `墳`                  | Icon to show under popup volume slider.                                                                                                               |
| `icons.track`         | `string/image`                        | ``                  | Icon to show next to track title.                                                                                                                     |
| `icons.album`         | `string/image`                        | ``                  | Icon to show next to album name.                                                                                                                      |
| `icons.artist`        | `string/image`                        | `ﴁ`                  | Icon to show next to artist name.                                                                                                                     |
| `icon_size`           | `integer`                             | `32`                 | Size to render icon at (image icons only).                                                                                                            |
| `cover_image_size`    | `integer`                             | `128`                | Size to render album art image at inside popup.                                                                                                       |
| `host`                | `string/image`                        | `localhost:6600`     | [MPD Only] TCP or Unix socket for the MPD server.                                                                                                     |
| `music_dir`           | `string/image`                        | `$HOME/Music`        | [MPD Only] Path to MPD server's music directory on disc. Required for album art.                                                                      |

See [here](images) for information on images.

<details>
<summary>JSON</summary>

```json
{
  "start": [
    {
      "type": "music",
      "player_type": "mpd",
      "format": "{title} / {artist}",
      "truncate": "end",
      "icons": {
        "play": "",
        "pause": ""
      },
      "music_dir": "/home/jake/Music"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[start]]
type = "music"
player_type = "mpd"
format = "{title} / {artist}"
music_dir = "/home/jake/Music"
truncate = "end"

[[start.icons]]
play = ""
pause = ""
```

</details>

<details>
<summary>YAML</summary>

```yaml
start:
  - type: "music"
    player_type: "mpd"
    format: "{title} / {artist}"
    truncate: "end"
    icons:
      play: ""
      pause: ""
    music_dir: "/home/jake/Music"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  start = [
    {
      type = "music"
      player_type = "mpd"
      format = "{title} / {artist}"
      truncate = "end"
      icons.play = ""
      icons.pause = ""
      music_dir = "/home/jake/Music"
    }
  ]
}
```

</details>

### Formatting Tokens

The following tokens can be used in the `format` config option,
and will be replaced with values from the currently playing track:

| Token        | Description                          |
|--------------|--------------------------------------|
| `{title}`    | Title                                |
| `{album}`    | Album name                           |
| `{artist}`   | Artist name                          |
| `{date}`     | Release date                         |
| `{track}`    | Track number                         |
| `{disc}`     | Disc number                          |
| `{genre}`    | Genre                                |
| `{duration}` | Duration in `mm:ss`                  |
| `{elapsed}`  | Time elapsed in `mm:ss`              |

## Styling

| Selector                            | Description                              |
|-------------------------------------|------------------------------------------|
| `#music`                            | Tray widget button                       |
| `#popup-music`                      | Popup box                                |
| `#popup-music #album-art`           | Album art image inside popup box         |
| `#popup-music #title`               | Track title container inside popup box   |
| `#popup-music #title .icon`         | Track title icon label inside popup box  |
| `#popup-music #title .label`        | Track title label inside popup box       |
| `#popup-music #album`               | Track album container inside popup box   |
| `#popup-music #album .icon`         | Track album icon label inside popup box  |
| `#popup-music #album .label`        | Track album label inside popup box       |
| `#popup-music #artist`              | Track artist container inside popup box  |
| `#popup-music #artist .icon`        | Track artist icon label inside popup box |
| `#popup-music #artist .label`       | Track artist label inside popup box      |
| `#popup-music #controls`            | Controls container inside popup box      |
| `#popup-music #controls #btn-prev`  | Previous button inside popup box         |
| `#popup-music #controls #btn-play`  | Play button inside popup box             |
| `#popup-music #controls #btn-pause` | Pause button inside popup box            |
| `#popup-music #controls #btn-next`  | Next button inside popup box             |
| `#popup-music #volume`              | Volume container inside popup box        |
| `#popup-music #volume #slider`      | Volume slider popup box                  |
| `#popup-music #volume .icon`        | Volume icon label inside popup box       |