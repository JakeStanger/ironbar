A Lua script to add a `.warning` class to the sysinfo module's CPU metre
when CPU usage crosses a threshold.

This uses Ironbar's built-in Lua engine to subscribe to the underlying client 
and modify the widget directly.

![CPU Graph](https://f.jstanger.dev/github/ironbar/lua-sysinfo-cpu-on.png)

## Configuration

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "sys_info",
      "format": [
        "{cpu_percent:0<2}% "
      ]
    }
  ]
}
```
</details>

<details>
<summary>YAML</summary>

```yaml
end:
- type: sys_info
  format:
  - '{cpu_percent:0<2}% '
```
</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "sys_info"
format = ["{cpu_percent:0<2}% "]
```
</details>

<details>
<summary>Corn</summary>

```corn
let {
    $sysinfo = { type = "sys_info" format = [ "{cpu_percent:0<2}% " ] }
} in {
    end = [ $sysinfo ]
}
```
</details>

## Script

```lua
-- init.lua
local CPU_THRESHOLD = 80

local bars = ironbar.get_bars()

--- Gets all `sysinfo` modules instaces across all bars
--- (assuming they're all in the end position).
local function get_sysinfos()
    local modules = {}
    for _, bar in ipairs(bars) do
        local module = bar:get_module("sys_info", Position.End)
        table.insert(modules, module)
    end

    return modules
end

local sys_client = ironbar.clients.sysinfo
local sysinfos = get_sysinfos()

sys_client:subscribe(function(ev)
    if ev.kind ~= "Cpu" then
        return
    end

    local percent = sys_client:cpu_percent():get("mean")

    if percent > CPU_THRESHOLD then
        for _, module in ipairs(sysinfos) do
            module:get_first_child():add_css_class("warning")
        end
    else
        for _, module in ipairs(sysinfos) do
            module:get_first_child():remove_css_class("warning")
        end
    end
end)
```

## Style

```css
.warning {
  background-color: #880000;
}
```