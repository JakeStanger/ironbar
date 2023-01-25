Displays currently playing song from your music player.
This module supports both MPRIS players and MPD servers.
Clicking on the widget opens a popout displaying info about the current song, album art
and playback controls.

in MPRIS mode, the widget will listen to all players and automatically detect/display the active one.

![Screenshot showing MPD widget with track playing with popout open](https://user-images.githubusercontent.com/5057870/184539664-a8f3ad5b-69c0-492d-a27d-82303c09a347.png)

## Configuration

> Type: `music`

|                | Type             | Default                     | Description                                                                      |
|----------------|------------------|-----------------------------|----------------------------------------------------------------------------------|
| `player_type`  | `mpris` or `mpd` | `mpris`                     | Whether to connect to MPRIS players or an MPD server.                            |
| `format`       | `string`         | `{icon} {title} / {artist}` | Format string for the widget. More info below.                                   |
| `icons.play`   | `string`         | ``                         | Icon to show when playing.                                                       |
| `icons.pause`  | `string`         | ``                         | Icon to show when paused.                                                        |
| `icons.volume` | `string`         | `墳`                         | Icon to show under popup volume slider.                                          |
| `host`         | `string`         | `localhost:6600`            | [MPD Only] TCP or Unix socket for the MPD server.                                |
| `music_dir`    | `string`         | `$HOME/Music`               | [MPD Only] Path to MPD server's music directory on disc. Required for album art. |

<details>
<summary>JSON</summary>

```json
{
  "start": [
    {
      "type": "music",
      "player_type": "mpd",
      "format": "{icon} {title} / {artist}",
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
format = "{icon} {title} / {artist}"
music_dir = "/home/jake/Music"

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
    format: "{icon} {title} / {artist}"
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
      format = "{icon} {title} / {artist}"
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
| `{icon}`     | Either `icons.play` or `icons.pause` |
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

| Selector                                 | Description                              |
|------------------------------------------|------------------------------------------|
| `#music`                                 | Tray widget button                       |
| `#popup-music`                           | Popup box                                |
| `#popup-music #album-art`                | Album art image inside popup box         |
| `#popup-music #title`                    | Track title container inside popup box   |
| `#popup-music #title .icon`              | Track title icon label inside popup box  |
| `#popup-music #title .label`             | Track title label inside popup box       |
| `#popup-music #album`                    | Track album container inside popup box   |
| `#popup-music #album .icon`              | Track album icon label inside popup box  |
| `#popup-music #album .label`             | Track album label inside popup box       |
| `#popup-music #artist`                   | Track artist container inside popup box  |
| `#popup-music #artist .icon`             | Track artist icon label inside popup box |
| `#popup-music #artist .label`            | Track artist label inside popup box      |
| `#popup-music #controls`                 | Controls container inside popup box      |
| `#popup-music #controls #btn-prev`       | Previous button inside popup box         |
| `#popup-music #controls #btn-play-pause` | Play/pause button inside popup box       |
| `#popup-music #controls #btn-next`       | Next button inside popup box             |
| `#popup-music #volume`                   | Volume container inside popup box        |
| `#popup-music #volume #slider`           | Volume slider popup box                  |
| `#popup-music #volume .icon`             | Volume icon label inside popup box       |