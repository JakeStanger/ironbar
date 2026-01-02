Allows you to compose custom modules consisting of multiple modules and widgets, including popups. 
Labels can display dynamic content from scripts, and buttons can interact with the bar or execute commands on click.

The module provides a set of utility widgets, such as containers, labels and buttons. 
In addition to these, you can also add any native module. 
Paired with the other custom modules such as Cairo, 
this provides a powerful declarative interface for constructing your own interfaces.

If you only intend to run a single script, prefer the [script](script) module, 
or [label](label) if you only need a single text label.

![Custom module with a button on the bar, and the popup open. The popup contains a header, shutdown button and restart button.](https://f.jstanger.dev/github/ironbar/modules/custom/power-menu.png)

![Custom module with a button on the bar, and the popup open. The popup contains a header, shutdown button and restart button.](https://f.jstanger.dev/github/ironbar/modules/custom/weather.png)

## Example

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
            <clock disable_popup="true" />
        </box>
    </popup>
</custom>
```

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
            { type = "clock" disable_popup = true }
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


## Configuration

> Type: `custom`

This module can be quite fiddly to configure as you effectively have to build a tree of widgets by hand.
It is well worth looking at the examples.

%properties%

### Widgets

Widgets are a special kind of module unique to the `custom` module. 

There are many widget types, each with their own config options.
You can think of these like HTML elements and their attributes.

They are configured in the same way as any other module,
and even support the same common module-level options.

To use a widget, set the `type` property to a widget name:

- [box](/modules/custom/widgets/box)
- [label](/modules/custom/widgets/label)
- [button](/modules/custom/widgets/button)
- [image](/modules/custom/widgets/image)
- [slider](/modules/custom/widgets/slider)
- [progress](/modules/custom/widgets/progress)

### Label Attributes

> [!NOTE]
> This is different to the `label` widget, although applies to it.

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


## Styling

Since the widgets are all custom, you can use their `name` and `class` attributes, then target them using `#name` and `.class`.

The following top-level selectors are always available:

| Selector        | Description                    |
|-----------------|--------------------------------|
| `.custom`       | Custom widget container.       |
| `.popup-custom` | Custom widget popup container. |

For more information on styling, please see the [styling guide](styling-guide).
