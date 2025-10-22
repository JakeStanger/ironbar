# Minimal

A basic bar suited to classic tiling WM workflows.

![Minimal theme bar with clock popup open](https://f.jstanger.dev/github/ironbar/themes/minimal.png)

Included modules:

- **Start**: `workspaces`
- **Center**: `focused`
- **End**: `battery`, `sys_info`, `tray`, `clock`

>[!NOTE]
> - The `battery` module is automatically hidden if no battery is detected.
> - The `sys_info` module shows CPU and memory usage percentages.

The included stylesheet provides a dark theme with monospace font, 
including the barebones essentials for the included modules only.

## Usage

This configuration is baked into Ironbar and can be used out of the box:

```shell
ironbar --config minimal
```

It is also possible to use the provided theme with another configuration:

```shell
ironbar --theme minimal
```

Alternatively, copy the config file of your preferred format, and the `style.css` into `~/.config/ironbar`.
These will be loaded automatically and provide a template to make your own changes.

>[!TIP]
> This is the default Ironbar configuration. 
> If no valid config file exists, ironbar will fall back to loading this.