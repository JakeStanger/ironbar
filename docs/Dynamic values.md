In some configuration locations, Ironbar supports dynamic values, 
meaning you can inject content into the bar from an external source.

Currently two dynamic content sources are supported - scripts and ironvars.

## Dynamic String

Dynamic strings can contain any mixture of static string elements, scripts and variables.

Scripts should be placed inside `{{double braces}}`. Both polling and watching scripts are supported.

Variables use the standard `#name` syntax. Variables cannot be placed inside scripts.

To use a literal hash, use `##`. This is only necessary outside of scripts.

Example:

```toml
label = "{{cat greeting.txt}}, #subject"
```

## Dynamic Boolean

Dynamic booleans can use a single source of either a script or variable to control a true/false value.

For scripts, you can just write these directly with no notation. 
Only polling scripts are supported. 
The script exit code is used, where `0` is `true` and any other code is `false.

For variables, use the standard `#name` notation. 
An empty string, `0` and `false` are treated as false. 
Any other value is true.

Example:

```toml
show_if = "exit 0" # script
show_if = "#show_module" # variable
```