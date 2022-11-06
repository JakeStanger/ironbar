The below config shows a module of each type being used.

The Corn format makes heavy use of variables 
to show how module configs can be easily referenced to improve readability 
and reduce config length when using multiple bars.

<details>
<summary>JSON</summary>

```json
{
  "start": [
    {
      "all_monitors": false,
      "name_map": {
        "1": "ﭮ",
        "2": "",
        "3": "",
        "Code": "",
        "Games": ""
      },
      "type": "workspaces"
    },
    {
      "favorites": [
        "firefox",
        "discord",
        "Steam"
      ],
      "icon_theme": "Paper",
      "show_icons": true,
      "show_names": false,
      "type": "launcher"
    }
  ],
  "end": [
    {
      "music_dir": "/home/jake/Music",
      "type": "mpd"
    },
    {
      "host": "chloe:6600",
      "type": "mpd"
    },
    {
      "path": "/home/jake/bin/phone-battery",
      "type": "script"
    },
    {
      "format": [
        "{cpu-percent}% ",
        "{memory-percent}% "
      ],
      "type": "sys-info"
    },
    {
      "type": "clock"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[start]]
all_monitors = false
type = 'workspaces'

[start.name_map]
1 = 'ﭮ'
2 = ''
3 = ''
Code = ''
Games = ''

[[start]]
icon_theme = 'Paper'
show_icons = true
show_names = false
type = 'launcher'
favorites = [
    'firefox',
    'discord',
    'Steam',
]

[[end]]
music_dir = '/home/jake/Music'
type = 'mpd'

[[end]]
host = 'chloe:6600'
type = 'mpd'

[[end]]
path = '/home/jake/bin/phone-battery'
type = 'script'

[[end]]
type = 'sys-info'
format = [
    '{cpu-percent}% ',
    '{memory-percent}% ',
]

[[end]]
type = 'clock'
```

</details>

<details>
<summary>YAML</summary>

```yaml
---
start:
  - all_monitors: false
    name_map:
      "1": ﭮ
      "2": 
      "3": 
      Code: 
      Games: 
    type: workspaces
  - favorites:
      - firefox
      - discord
      - Steam
    icon_theme: Paper
    show_icons: true
    show_names: false
    type: launcher
end:
  - music_dir: /home/jake/Music
    type: mpd
  - host: "chloe:6600"
    type: mpd
  - path: /home/jake/bin/phone-battery
    type: script
  - format:
      - "{cpu-percent}% "
      - "{memory-percent}% "
    type: sys-info
  - type: clock
```
</details>

<details>
<summary>Corn</summary>

```corn
let {
    $workspaces = {
        type = "workspaces"
        all_monitors = false
        name_map = {
            1 = "ﭮ"
            2 = ""
            3 = ""
            Games = ""
            Code = ""
        }
    }

    $launcher = {
        type = "launcher"
        favorites = ["firefox" "discord" "Steam"]
        show_names = false
        show_icons = true
        icon_theme = "Paper"
    }

    $mpd_local = { type = "mpd" music_dir = "/home/jake/Music" }
    $mpd_server = { type = "mpd" host = "chloe:6600" }

    $sys_info = {
        type = "sys-info"
        format = ["{cpu-percent}% " "{memory-percent}% "]
    }

    $tray = { type = "tray" }
    $clock = { type = "clock" }
    
    $phone_battery = {
    type = "script"
    path = "/home/jake/bin/phone-battery"
    }
    
    $start = [ $workspaces $launcher ]
    $end = [ $mpd_local $mpd_server $phone_battery $sys_info $clock ]
}
in {
    start = $start
    end = $end
}
```
</details>