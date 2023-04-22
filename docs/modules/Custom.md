Allows you to compose custom modules consisting of multiple widgets, including popups. 
Labels can display dynamic content from scripts, and buttons can interact with the bar or execute commands on click.

![Custom module with a button on the bar, and the popup open. The popup contains a header, shutdown button and restart button.](https://f.jstanger.dev/github/ironbar/custom-power-menu.png?raw)

## Configuration

> Type: `custom`

This module can be quite fiddly to configure as you effectively have to build a tree of widgets by hand.
It is well worth looking at the examples.

### `Widget`

There are many widget types, each with their own config options. 
You can think of these like HTML elements and their attributes.

Every widget has the following options available; `type` is mandatory. 
You can also add common [module-level options](https://github.com/JakeStanger/ironbar/wiki/configuration-guide#32-module-level-options) on a widget.

| Name    | Type                                                              | Default | Description                   |
|---------|-------------------------------------------------------------------|---------|-------------------------------|
| `type`  | `box` or `label` or `button` or `image` or `slider` or `progress` | `null`  | Type of GTK widget to create. |
| `name`  | `string`                                                          | `null`  | Widget name.                  |
| `class` | `string`                                                          | `null`  | Widget class name.            |

#### Box

A container to place nested widgets inside.

> Type: `box`

| Name          | Type                                               | Default      | Description                                                       |
|---------------|----------------------------------------------------|--------------|-------------------------------------------------------------------|
| `orientation` | `horizontal` or `vertical` (shorthand: `h` or `v`) | `horizontal` | Whether child widgets should be horizontally or vertically added. |
| `widgets`     | `Widget[]`                                         | `[]`         | List of widgets to add to this box.                               |

#### Label

A text label. Pango markup and embedded scripts are supported.

> Type `label`

| Name    | Type     | Default      | Description                                                         |
|---------|----------|--------------|---------------------------------------------------------------------|
| `label` | `string` | `horizontal` | Widget text label. Pango markup and embedded scripts are supported. |

#### Button

A clickable button, which can run a command when clicked.

> Type `button`

| Name       | Type               | Default      | Description                                                         |
|------------|--------------------|--------------|---------------------------------------------------------------------|
| `label`    | `string`           | `horizontal` | Widget text label. Pango markup and embedded scripts are supported. |
| `on_click` | `string [command]` | `null`       | Command to execute. More on this [below](#commands).                |

#### Image

An image or icon from disk or http.

> Type `image`

| Name   | Type      | Default | Description                                                                                 |
|--------|-----------|---------|---------------------------------------------------------------------------------------------|
| `src`  | `image`   | `null`  | Image source. See [here](images) for information on images. Embedded scripts are supported. |
| `size` | `integer` | `null`  | Width/height of the image. Aspect ratio is preserved.                                       |

#### Slider

A draggable slider.

> Type: `slider`

Note that `on_change` will provide the **floating point** value as an argument. 
If your input program requires an integer, you will need to round it.

| Name          | Type                                               | Default      | Description                                                                  |
|---------------|----------------------------------------------------|--------------|------------------------------------------------------------------------------|
| `src`         | `image`                                            | `null`       | Image source. See [here](images) for information on images.                  |
| `size`        | `integer`                                          | `null`       | Width/height of the image. Aspect ratio is preserved.                        |
| `orientation` | `horizontal` or `vertical` (shorthand: `h` or `v`) | `horizontal` | Orientation of the slider.                                                   |
| `value`       | `Script`                                           | `null`       | Script to run to get the slider value. Output must be a valid number.        | 
| `on_change`   | `string [command]`                                 | `null`       | Command to execute when the slider changes. More on this [below](#commands). | 
| `min`         | `float`                                            | `0`          | Minimum slider value.                                                        | 
| `max`         | `float`                                            | `100`        | Maximum slider value.                                                        | 
| `length`      | `integer`                                          | `null`       | Slider length. GTK will automatically size if left unset.                    |

The example slider widget below shows a volume control for MPC, 
which updates the server when changed, and polls the server for volume changes to keep the slider in sync.

```corn
$slider = { 
    type = "custom" 
    bar = [
        {
            type = "slider"
            length = 100
            max = 100
            on_change="!mpc volume ${0%.*}"
            value = "200:mpc volume | cut -d ':' -f2 | cut -d '%' -f1"
        }
    ] 
}
```

#### Progress

A progress bar.

> Type: `progress`

Note that `value` expects a numeric value **between 0-`max`** as output.

| Name          | Type                                               | Default      | Description                                                                     |
|---------------|----------------------------------------------------|--------------|---------------------------------------------------------------------------------|
| `src`         | `image`                                            | `null`       | Image source. See [here](images) for information on images.                     |
| `size`        | `integer`                                          | `null`       | Width/height of the image. Aspect ratio is preserved.                           |
| `orientation` | `horizontal` or `vertical` (shorthand: `h` or `v`) | `horizontal` | Orientation of the slider.                                                      |
| `value`       | `Script`                                           | `null`       | Script to run to get the progress bar value. Output must be a valid percentage. |
| `max`         | `float`                                            | `100`        | Maximum progress bar value.                                                     | 
| `length`      | `integer`                                          | `null`       | Slider length. GTK will automatically size if left unset.                       |

The example below shows progress for the current playing song in MPD, 
and displays the elapsed/length timestamps as a label above:

```corn
$progress = { 
    type = "custom" 
    bar = [
        {
            type = "progress"
            value = "500:mpc | sed -n 2p | awk '{ print $4 }' | grep -Eo '[0-9]+' || echo 0"
            label = "{{500:mpc | sed -n 2p | awk '{ print $3 }'}} elapsed"
            length = 200
        }
    ] 
}
```

### Label Attributes

> ℹ This is different to the `label` widget, although applies to it.

Any widgets with a `label` attribute support embedded scripts, 
meaning you can interpolate text from scripts to dynamically show content. 

This can be done by including scripts in `{{double braces}}` using the shorthand script syntax.

For example, the following label would output your system uptime, updated every 30 seconds.

```
Uptime: {{30000:uptime -p | cut -d ' ' -f2-}}
```

Both polling and watching mode are supported. For more information on script syntax, see [here](scripts).

### Commands

Buttons can execute commands that interact with the bar, 
as well as any arbitrary shell command.

To execute shell commands, prefix them with an `!`. 
For example, if you want to run `~/.local/bin/my-script.sh` on click, 
you'd set `on_click` to `!~/.local/bin/my-script.sh`.

Some widgets provide a value when they run the command, such as `slider`.
This is passed as an argument and can be accessed using `$0`.

The following bar commands are supported:

- `popup:toggle`
- `popup:open`
- `popup:close`

---

XML is arguably better-suited and easier to read for this sort of markup, 
but currently is not supported.
Nonetheless, it may be worth comparing the examples to the below equivalent
to help get your head around what's going on:


```xml
<?xml version="1.0" encoding="utf-8" ?>
<custom class="power-menu">
    <bar>
        <button name="power-btn" label="" on_click="popup:toggle"/>
    </bar>
    <popup>
        <box orientation="vertical">
            <label name="header" label="Power menu" />
            <box>
                <button class="power-btn" label="" on_click="!shutdown now" />
                <button class="power-btn" label="" on_click="!reboot" />
            </box>
            <label name="uptime" label="Uptime: {{30000:uptime -p | cut -d ' ' -f2-}}" />
        </box>
    </popup>
</custom>
```

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "clock"
    },
    {
      "bar": [
        {
          "on_click": "popup:toggle",
          "label": "",
          "name": "power-btn",
          "type": "button"
        }
      ],
      "class": "power-menu",
      "popup": [
        {
          "orientation": "vertical",
          "type": "box",
          "widgets": [
            {
              "label": "Power menu",
              "name": "header",
              "type": "label"
            },
            {
              "type": "box",
              "widgets": [
                {
                  "class": "power-btn",
                  "on_click": "!shutdown now",
                  "label": "<span font-size='40pt'></span>",
                  "type": "button"
                },
                {
                  "class": "power-btn",
                  "on_click": "!reboot",
                  "label": "<span font-size='40pt'></span>",
                  "type": "button"
                }
              ]
            },
            {
              "label": "Uptime: {{30000:uptime -p | cut -d ' ' -f2-}}",
              "name": "uptime",
              "type": "label"
            }
          ]
        }
      ],
      "type": "custom"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = 'clock'

[[end]]
class = 'power-menu'
type = 'custom'

[[end.bar]]
on_click = 'popup:toggle'
label = ''
name = 'power-btn'
type = 'button'

[[end.popup]]
orientation = 'vertical'
type = 'box'

[[end.popup.widgets]]
label = 'Power menu'
name = 'header'
type = 'label'

[[end.popup.widgets]]
type = 'box'

[[end.popup.widgets.widgets]]
class = 'power-btn'
on_click = '!shutdown now'
label = '''<span font-size='40pt'></span>'''
type = 'button'

[[end.popup.widgets.widgets]]
class = 'power-btn'
on_click = '!reboot'
label = '''<span font-size='40pt'></span>'''
type = 'button'

[[end.popup.widgets]]
label = '''Uptime: {{30000:uptime -p | cut -d ' ' -f2-}}'''
name = 'uptime'
type = 'label'
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
- type: clock
- bar:
  - on_click: popup:toggle
    label: 
    name: power-btn
    type: button
  class: power-menu
  popup:
  - orientation: vertical
    type: box
    widgets:
    - label: Power menu
      name: header
      type: label
    - type: box
      widgets:
      - class: power-btn
        on_click: '!shutdown now'
        label: <span font-size='40pt'></span>
        type: button
      - class: power-btn
        on_click: '!reboot'
        label: <span font-size='40pt'></span>
        type: button
    - label: 'Uptime: {{30000:uptime -p | cut -d '' '' -f2-}}'
      name: uptime
      type: label
  type: custom
```

</details>

<details>
<summary>Corn</summary>

```corn
let {
    $button = { type = "button" name="power-btn" label = "" on_click = "popup:toggle" }

    $popup = {
        type = "box"
        orientation = "vertical"
        widgets = [
            { type = "label" name = "header" label = "Power menu" }
            {
                type = "box"
                widgets = [
                    { type = "button" class="power-btn" label = "<span font-size='40pt'></span>" on_click = "!shutdown now" }
                    { type = "button" class="power-btn" label = "<span font-size='40pt'></span>" on_click = "!reboot" }
                ]
            }
            { type = "label" name = "uptime" label = "Uptime: {{30000:uptime -p | cut -d ' ' -f2-}}" }
        ]
    }

    $power_menu = {
        type = "custom"
        class = "power-menu"

        bar = [ $button ]
        popup = [ $popup ]

        tooltip = "Up: {{30000:uptime -p | cut -d ' ' -f2-}}"
    }
} in {
    end = [ $power_menu ]
}
```

</details>

## Styling

Since the widgets are all custom, you can use the `name` and `class` attributes, then target them using `#name` and `.class`.

The following top-level selector is always available:

| Selector  | Description             |
|-----------|-------------------------|
| `#custom` | Custom widget container |