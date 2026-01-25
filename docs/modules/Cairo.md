Allows you to render custom content using the Lua and the Cairo drawing library.
This is an advanced feature which provides a powerful escape hatch, allowing you to fetch data and render anything
using an embedded scripting environment.

Scripts are automatically hot-reloaded.

> [!NOTE]
> The Lua engine uses LuaJIT 5.1, and requires the use of a library called `lgi`.
> Ensure you have the correct lua-lgi package installed.

![Circle clock](https://f.jstanger.dev/github/ironbar/cairo-clock.png)

## Configuration

> Type: `cairo`

| Name               | Type      | Default | Description                                        |
|--------------------|-----------|---------|----------------------------------------------------|
| `path`             | `string`  | `null`  | The path to the Lua script to load.                |
| `frequency`        | `float`   | `200`   | The number of milliseconds between each draw call. |
| `width`            | `integer` | `42`    | The canvas width in pixels.                        |
| `height`           | `integer` | `42`    | The canvas height in pixels.                       |

<details>
<summary>JSON</summary>

```json
{
  "center": [
    {
      "type": "cairo",
      "path": ".config/ironbar/clock.lua",
      "frequency": 100,
      "width": 300,
      "height": 300
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[center]]
type = "cairo"
path = ".config/ironbar/clock.lua"
frequency = 100
width = 300
height = 300
```
</details>

<details>
<summary>YAML</summary>

```yaml
center:
- type: cairo
  path: .config/ironbar/clock.lua
  frequency: 100
  width: 300
  height: 300
```

</details>

<details>
<summary>Corn</summary>

```corn
let { 
    $config_dir = ".config/ironbar" 
    $cairo = { 
        type = "cairo" 
        path = "$config_dir/clock.lua" 
        frequency = 100 
        width = 300 
        height = 300 
    } 
} in { 
    center = [ $cairo ] 
}
```

</details>

### Script

Every script must contain a function called `draw`. 
This takes a three parameters:
- Cairo context (required)
- Width of the drawing area (can be omitted)
- Height of the drawing area (can be omitted)

Outside of this, you can do whatever you like. 
The full lua `stdlib` is available, and you can load in additional system packages as desired.

Additionally there is basic access to the ironbar via the `ironbar` global:
- `ironbar.config_dir`: Absolute path to the configuration directory. This can be used for relative file imports, e.g.:
  ```lua
   local_module = dofile(ironbar.config_dir .. "local_mod.lua")`
   ```
- `ironbar:log_debug(msg)`, `ironbar:log_info(msg)`, `ironbar:log_warn(msg)`,`ironbar:log_error(msg)`: Write a log message.
- `ironbar:unixtime()`: Returns high-resolution unixtime (stdlib only offers second-resolution). Will return a table:
  - `sec`: Seconds since unix-epoch with fractions
  - `subsec_millis`: Sub-second milliseconds as integer
  - `subsec_micros`: Sub-second microseconds as integer
- `ironbar:var_get(key)`: Get an ironbar variable, e.g.
  ```lua
  memory_free = ironbar:var_get("sysinfo.memory_free")
  ```
- `ironbar:var_list(namespace)`: Get all variables of a namespace as table (non-recursive), e.g.:
  ```lua
  memory_free = ironbar:var_list("sysinfo")["memory_free"]
  ```


The most basic example, which draws a red square, can be seen below:

```lua
function draw(cr) 
    cr:set_source_rgb(1.0, 0.0, 0.0)
    cr:paint()
end
```

A longer example, used to create the clock in the image at the top of the page, is shown below:

<details>
<summary>Circle clock</summary>

```lua
function get_ms()
    return ironbar:unixtime().subsec_millis / 1000
    -- Only using the stdlib would require something like:
    -- local ms = tostring(io.popen('date +%s%3N'):read('a')):sub(-4, 9999)
    -- return tonumber(ms) / 1000
end

function draw(cr, width, height)
    local center_x = width / 2
    local center_y = height / 2
    local radius = math.min(width, height) / 2 - 20

    local date_table = os.date("*t")

    local hours = date_table["hour"]
    local minutes = date_table["min"]
    local seconds = date_table["sec"]
    local ms = get_ms()


    local label_seconds = seconds
    seconds = seconds + ms

    local hours_str = tostring(hours)
    if string.len(hours_str) == 1 then
        hours_str = "0" .. hours_str
    end

    local minutes_str = tostring(minutes)
    if string.len(minutes_str) == 1 then
        minutes_str = "0" .. minutes_str
    end

    local seconds_str = tostring(label_seconds)
    if string.len(seconds_str) == 1 then
        seconds_str = "0" .. seconds_str
    end

    local font_size = radius / 5.5

    cr:set_source_rgb(1.0, 1.0, 1.0)

    cr:move_to(center_x - font_size * 2.5 + 10, center_y + font_size / 2.5)
    cr:set_font_size(font_size)
    cr:show_text(hours_str .. ':' .. minutes_str .. ':' .. seconds_str)
    cr:stroke()

    if hours > 12 then
        hours = hours - 12
    end

    local line_width = radius / 8
    local start_angle = -math.pi / 2

    local end_angle = start_angle + ((hours + minutes / 60 + seconds / 3600) / 12) * 2 * math.pi
    cr:set_line_width(line_width)
    cr:arc(center_x, center_y, radius, start_angle, end_angle)
    cr:stroke()

    end_angle = start_angle + ((minutes + seconds / 60) / 60) * 2 * math.pi
    cr:set_line_width(line_width)
    cr:arc(center_x, center_y, radius * 0.8, start_angle, end_angle)
    cr:stroke()

    if seconds == 0 then
        seconds = 60
    end

    end_angle = start_angle + (seconds / 60) * 2 * math.pi
    cr:set_line_width(line_width)
    cr:arc(center_x, center_y, radius * 0.6, start_angle, end_angle)
    cr:stroke()

    return 0
end
```

</details>

> [!TIP]
> The C documentation for the Cairo context interface can be found [here](https://www.cairographics.org/manual/cairo-cairo-t.html).
> The Lua interface provides a slightly friendlier API which restructures things slightly.
> The `cairo_` prefix is dropped, and the `cairo_t *cr` parameters are replaced with a namespaced call. 
> For example, `cairo_paint (cairo_t *cr)` becomes `cr:paint()`

> [!TIP]
> Ironbar's Cairo module has similar functionality to the popular Conky program.
> You can often re-use scripts with little work. 

### Initialization

You can optionally create an `init.lua` file in your config directory. 
Any code in here will be executed once, on bar startup. 

As variables and functions are global by default in Lua,
this provides a mechanism for sharing code between multiple modules.

## Styling

| Selector | Description             |
|----------|-------------------------|
| `.cairo` | Cairo widget container. |

For more information on styling, please see the [styling guide](styling-guide).