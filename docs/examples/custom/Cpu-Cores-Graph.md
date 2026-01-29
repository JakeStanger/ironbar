Creates an inline bar chart of CPU usage per core

![CPU Graph](https://f.jstanger.dev/github/ironbar/cario-cpu-graph.png)


## Configuration

<details>
<summary>JSON</summary>

```json
{
  "height": 20,
  "end": [
    {
      "type": "cairo",
      "path": ".config/ironbar/cpu_graph.lua",
      "frequency": 500,
      "width": 320,
      "height": 20
    },
    {
      "type": "sys_info",
      "format": [
        "{load_average_1}/{load_average_5}/{load_average_15}"
      ],
      "interval": {
        "cpu": 1
      }
    }
  ]
}
```

</details>
<details>
<summary>TOML</summary>

```toml
height = 20

[[end]]
type = "cairo"
path = ".config/ironbar/cpu_graph.lua"
frequency = 500
width = 320
height = 20

[[end]]
type = "sys_info"
format = [
    "{load_average_1}/{load_average_5}/{load_average_15}",
]

[end.interval]
cpu = 1
```

</details>
<details>
<summary>YAML</summary>

```yaml
height: 20
end:
- type: cairo
  path: .config/ironbar/cpu_graph.lua
  frequency: 500
  width: 320
  height: 20
- type: sys_info
  format:
  - "{load_average_1}/{load_average_5}/{load_average_15}"
  interval:
    cpu: 1
```

</details>
<details>
<summary>Corn</summary>

```corn
let {
    $sys_info = {
        type = "sys_info"
        format = ["{load_average_1}/{load_average_5}/{load_average_15}"]
        interval.cpu = 1
    }

    $config_dir = "$env_HOME/.config/ironbar"
    $cpu_graph = {
        type = "cairo"
        path = "$config_dir/cpu_graph.lua"
        frequency = 500
        width = 320
        height = 20
    }
} in {
    height = 20
    
    end = [ $cpu_graph $sys_info ]
}
```

</details>

## Script

`~/.config/ironbar/cpu_graph.lua`:
```lua
local function text_left_center(cr, x, y, text)
  extent = cr:text_extents(text)
  cr:move_to(x, y + extent.height / 2 + 2)
  cr:show_text(text)
  return extent.width
end

local function text_right_center(cr, x, y, text)
  extent = cr:text_extents(text)
  cr:move_to(x - extent.width, y + extent.height / 2 + 2)
  cr:show_text(text)
  return extent.width
end

local function draw(cr, area_width, area_height)
    -- Number of CPU cores to display
    local num_cores = 32
    local draw_height = area_height - 4
    local mean_cpu_frequency = ironbar:var_get("sysinfo.cpu_frequency.mean")
    local cpu_percent = ironbar:var_list("sysinfo.cpu_percent")
    local mean_cpu_percent = cpu_percent["mean"]

    -- Adjust according to preference.
    -- The used icon requires a Nerd Font though
    cr:select_font_face("Hack Nerd Font")
    cr:set_font_size(draw_height - 4)

    -- Color is set by overall usage
    -- Using temperature might be an alternative
    if mean_cpu_percent > 80 then
      cr:set_source_rgb(1.0, 0.0, 0.0)
    elseif mean_cpu_percent > 50 then
      cr:set_source_rgb(1.0, 1.0, 0.0)
    else
      cr:set_source_rgb(1.0, 1.0, 1.0)
    end

    local header_width = text_left_center(cr, 0, draw_height / 2, "\u{eeb2}") + 5
    local cpu_info = string.format("%3.1f%% %2.2fGHz", mean_cpu_percent, mean_cpu_frequency / 1000000000.0)
    local tail_width = text_right_center(cr, area_width, draw_height / 2, cpu_info) + 5

    local bar_width = (area_width - header_width - tail_width - 4) / num_cores

    for i = 0, num_cores - 1 do
      local core_percent = cpu_percent["cpu" .. i]
      local height = math.max(math.ceil(core_percent * draw_height / 100.0), 1)

      cr:rectangle(i * bar_width + header_width + 2, area_height - height - 2, bar_width, height)
      cr:fill()
    end
end

return draw
```
