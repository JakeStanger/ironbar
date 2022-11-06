Allows you to compose custom modules consisting of multiple widgets, including popups. 
Buttons can interact with the bar or execute commands on click.

![Custom module with a button on the bar, and the popup open. The popup contains a header, shutdown button and restart button.](https://user-images.githubusercontent.com/5057870/196058785-042ef171-7e77-4d5c-921a-eca03c6424bd.png)

## Configuration

> Type: `custom`

This module can be quite fiddly to configure as you effectively have to build a tree of widgets by hand.
It is well worth looking at the examples.

| Name    | Type       | Default | Description                          |
|---------|------------|---------|--------------------------------------|
| `class` | `string`   | `null`  | Container class name.                |
| `bar`   | `Widget[]` | `null`  | List of widgets to add to the bar.   |
| `popup` | `Widget[]` | `[]`    | List of widgets to add to the popup. |

### `Widget`

| Name          | Type                         | Default      | Description                                                               |
|---------------|------------------------------|--------------|---------------------------------------------------------------------------|
| `widget_type` | `box` or `label` or `button` | `null`       | Type of GTK widget to create.                                             |
| `name`        | `string`                     | `null`       | Widget name.                                                              |
| `class`       | `string`                     | `null`       | Widget class name.                                                        |
| `label`       | `string`                     | `null`       | [`label` and `button`] Widget text label. Pango markup supported.         |
| `exec`        | `string`                     | `null`       | [`button`] Command to execute. More on this [below](#commands).           |
| `orientation` | `horizontal` or `vertical`   | `horizontal` | [`box`] Whether child widgets should be horizontally or vertically added. |
| `widgets`     | `Widget[]`                   | `[]`         | [`box`] List of widgets to add to this box.                               |

### Commands

Buttons can execute commands that interact with the bar, 
as well as any arbitrary shell command.

To execute shell commands, prefix them with an `!`. 
For example, if you want to run `~/.local/bin/my-script.sh` on click, 
you'd set `exec` to `!~/.local/bin/my-script.sh`.

The following bar commands are supported:

- `popup:toggle`
- `popup:open`
- `popup:close`

XML is arguably better-suited and easier to read for this sort of markup, 
but currently not supported.
Nonetheless, it may be worth comparing the examples to the below equivalent
to help get your head around what's going on:


```xml
<?xml version="1.0" encoding="utf-8" ?>
<custom class="power-menu">
    <bar>
        <button name="power-btn" label="" exec="popup:toggle"/>
    </bar>
    <popup>
        <box orientation="vertical">
            <label name="header" label="Power menu" />
            <box>
                <button class="power-btn" label="" exec="!shutdown now" />
                <button class="power-btn" label="" exec="!reboot" />
            </box>
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
      "type": "custom",
      "bar": [
        {
          "exec": "popup:toggle",
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
                  "exec": "!shutdown now",
                  "label": "<span font-size='40pt'></span>",
                  "type": "button"
                },
                {
                  "class": "power-btn",
                  "exec": "!reboot",
                  "label": "<span font-size='40pt'></span>",
                  "type": "button"
                }
              ]
            }
          ]
        }
      ]
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
class = 'power-menu'
type = 'custom'

[[end.bar]]
exec = 'popup:toggle'
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
exec = '!shutdown now'
label = '''<span font-size='40pt'></span>'''
type = 'button'

[[end.popup.widgets.widgets]]
class = 'power-btn'
exec = '!reboot'
label = '''<span font-size='40pt'></span>'''
type = 'button'
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
- type: custom
  bar:
  - exec: popup:toggle
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
        exec: '!shutdown now'
        label: <span font-size='40pt'></span>
        type: button
      - class: power-btn
        exec: '!reboot'
        label: <span font-size='40pt'></span>
        type: button
```

</details>

<details>
<summary>Corn</summary>

```corn
let {
    $power_menu = {
        type = "custom"
        class = "power-menu"

        bar = [ { type = "button" name="power-btn" label = "" exec = "popup:toggle" } ]

        popup = [ {
            type = "box"
            orientation = "vertical"
            widgets = [
                { type = "label" name = "header" label = "Power menu" }
                {
                    type = "box"
                    widgets = [
                        { type = "button" class="power-btn" label = "<span font-size='40pt'></span>" exec = "!shutdown now" }
                        { type = "button" class="power-btn" label = "<span font-size='40pt'></span>" exec = "!reboot" }
                    ]
                }
            ]
        } ]
    }
} in {
    end = [ $power_menu ]
}
```

</details>

## Styling

Since the widgets are all custom, you can target them using `#name` and `.class`.

| Selector  | Description             |
|-----------|-------------------------|
| `#custom` | Custom widget container |