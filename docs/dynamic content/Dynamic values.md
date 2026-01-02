In some configuration locations, Ironbar supports dynamic values, 
meaning you can inject content into the bar from an external source.

Currently two dynamic content sources are supported - [scripts](scripts) (via shorthand syntax) and [ironvars](ironvars).

## Dynamic String

Dynamic strings can contain any mixture of static string elements, scripts and variables.

- Scripts should be placed inside `{{double braces}}`. Both polling and watching scripts are supported.
- Variables use the standard `#name` syntax. Variables cannot be placed inside scripts.
- To use a literal hash, use `##`. This is only necessary outside of scripts.

**Example:**

```toml
label = "{{cat greeting.txt}}, #subject"
```

Scripts can be used to represent information which is quick to fetch,
while ironvars are better suited to data that is more complex/expensive to fetch or calculate.

An example script might be to display system uptime, or a pending update count:

```toml
label = "Uptime: {{uptime -p | cut -d ' ' -f2-}}"
```

Variables tend to come in more for externally controlled data.
You might for example have a [script and module](weather) that fetches the weather,
and wish to display the data:

```
label = "Weather: #weather_cond | #weather_temp"
```

## Dynamic Boolean

Dynamic booleans can use a single source of either a script or variable to control a true/false value.

For scripts, you can just write these directly with no notation. 
Only polling scripts are supported. 
The script exit code is used, where `0` is `true` and any other code is `false`.

For variables, use the standard `#name` notation. 
An empty string, `0` and `false` are treated as false. 
Any other value is true.

**Example:**

```toml
show_if = "exit 0" # script
show_if = "#show_module" # variable
```

This can be used for example to show/hide a battery based on whether one is present.

```corn
{
  end = [ { type = "battery" show_if = "[ -f /sys/class/power_supply/BAT0 ]" } ]
}
```

Another use is to show/hide modules when another one is hovered or clicked.

```corn
let {
    $clock = { 
        type = "clock" 
        format = "%H:%M:%S"
        on_mouse_enter = "ironbar var set clock_state true" 
        on_mouse_exit = "ironbar var set clock_state false" 
    }
    
    $clock_extra = {
        type = "clock"
        format = "%d/%m/%Y"
        show_if = "#clock_state"
    }
} in {
    end = [ $clock_extra $clock ]
}
```
