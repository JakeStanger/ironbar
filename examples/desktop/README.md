# Desktop

A feature-rich bar providing an experience closer to a more traditional desktop environment bar.

>[!TIP]
> This configuration works best in environments which support blur.

![Desktop theme bar with music popup open](https://f.jstanger.dev/github/ironbar/themes/desktop.png)

Included modules:

- **Start**: `menu`, `workspaces`, `launcher`
- **Center**: `music`
- **End**: `battery`, `sys_info`, `clipboard`, `volume`, `custom` (power menu), `tray`, `clock`, `notifications`

>[!NOTE]
> - The `battery` module is automatically hidden if no battery is detected.
> - The `sys_info` module shows CPU and memory usage percentages.
> - The `custom` power menu provides a popup with shutdown/restart buttons.
> - The `notifications` module is automatically hidden if SwayNC is not detected.

The included stylesheet features a modern design, taking inspiration from KDE Plasma 6.
The theme makes use of transparency, best suited to a compositor with blur support. 
This is a dark theme.

The bar is larger and more spacious than a traditional WM status bar,
making it better suited to mouse-oriented users.

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
