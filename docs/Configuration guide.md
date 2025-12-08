By default, you get a single bar at the bottom of all your screens.
To change that, you'll unsurprisingly need a config file.

This page details putting together the skeleton for your config to get you to a stage where you can start configuring
modules.
It may look long and overwhelming, but that is just because the bar supports a lot of scenarios!

If you want to see some ready-to-go config files check
the [examples folder](https://github.com/JakeStanger/ironbar/tree/master/examples)
and the example pages in the sidebar.

The examples make use of [Nerd Fonts](https://www.nerdfonts.com/#home) for displaying symbols.

## 1. Create config file

The config file lives inside the `ironbar` directory in your XDG_CONFIG_DIR, which is usually `~/.config/ironbar`.

Ironbar supports a range of configuration formats, so you can pick your favourite:

- `config.json`
- `config.toml`
- `config.yaml`
- `config.corn` (Includes variable support for re-using blocks.
  See [here](https://github.com/jakestanger/corn) for info)

You can also override the default config path using the `IRONBAR_CONFIG` environment variable.

A hosted schema is available for the latest Git version ~~and each versioned release~~.
JSON and YAML both support schema checking by adding the `$schema` key 
to the top level of your config.

- master: `https://f.jstanger.dev/github/ironbar/schema.json`
- ~~release: `https://f.jstanger.dev/github/ironbar/schema-v0.16.0.json`~~ *(Not released yet)*

## 2. Pick your use-case

Ironbar gives you a few ways to configure the bar to suit your needs.
This allows you to keep your config simple and relatively flat if your use-case is simple,
and make it more complex if required.

### a) I want the same bar across all monitors

Place the bar config inside the top-level object. This is automatically applied to each of your monitors.

<details>
<summary>JSON</summary>

```json
{
  "position": "bottom",
  "height": 42,
  "start": [],
  "center": [],
  "end": []
}
```

</details>

<details>
<summary>TOML</summary>

```toml
position = "bottom"
height = 42
start = []
center = []
end = []
```

</details>

<details>
<summary>YAML</summary>

```yaml
position: "bottom"
height: 42
start: [ ]
center: [ ]
end: [ ]
```

</details>

<details>
<summary>Corn</summary>

```
{
  position = "bottom"
  height = 42
  start = []
  center = []
  end = []
}
```

</details>

### b) I want my config to differ across one or more monitors

Create a map/object called `monitors` inside the top-level object.
Each of the map's keys should be an output name of description,
and each value should be an object containing the bar config.

Output names can be supplied in two formats:

  - Connector names (`DP-1`, `HDMI-2`)
  - Descriptions (`ASUSTek COMPUTER INC PA278QV M4LMQS060475`).
    A `starts_with` is applied allowing you to omit part of the description if convenient.

You can still define a top-level "default" config to use for unspecified monitors.
Alternatively, leave the top-level `start`, `center` and `end` keys null to hide bars on unspecified monitors.

> [!TIP]
> To find your output names, run `wayland-info | grep wl_output -A1`.

<details>
<summary>JSON</summary>

```json
{
  "monitors": {
    "DP-1": {
      "start": []
    },
    "DP-2": {
      "position": "bottom",
      "height": 30,
      "start": []
    }
  }
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[monitors]

[monitors.DP-1]
start = []

[monitors.DP-2]
position = "bottom"
height = 30
start = []
```

</details>

<details>
<summary>YAML</summary>

```yaml
monitors:
  DP-1:
    start: [ ]
  DP-2:
    position: "bottom"
    height: 30
    start: [ ]
```

</details>

<details>
<summary>Corn</summary>

```
{
  monitors.DP-1.start = []
  monitors.DP-2 = {
    position = "bottom"
    height = 30
    start = []
  }
}
```

</details>

### c) I want one or more monitors to have multiple bars

Create a map/object called `monitors` inside the top-level object.
Each of the map's keys should be an output name.
If you want the screen to have multiple bars, use an array of bar config objects.
If you want the screen to have a single bar, use an object.

Output names can be supplied in two formats:

- Connector names (`DP-1`, `HDMI-2`)
- Descriptions (`ASUSTek COMPUTER INC PA278QV M4LMQS060475`).
  A `starts_with` is applied allowing you to omit part of the description if convenient.


To find your output names, run `wayland-info | grep wl_output -A1`.

<details>
<summary>JSON</summary>

```json
{
  "monitors": {
    "DP-1": [
      {
        "start": []
      },
      {
        "position": "top",
        "start": []
      }
    ],
    "DP-2": {
      "position": "bottom",
      "height": 30,
      "start": []
    }
  }
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[monitors]

[[monitors.DP-1]]
start = []

[[monitors.DP-2]]
position = "top"
start = []

[monitors.DP-2]
position = "bottom"
height = 30
start = []
```

</details>

<details>
<summary>YAML</summary>

```yaml
monitors:
  DP-1:
    - start: [ ]
    - position: "top"
      start: [ ]
  DP-2:
    position: "bottom"
    height: 30
    start: [ ]
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  monitors.DP-1 = [
    { start = [] }
    { position = "top" start = [] }
  ]
  monitors.DP-2 = {
    position = "bottom"
    height = 30
    start = []
  }
}
```

</details>

## 3. Write your bar config(s)

Once you have the basic config structure set up, it's time to actually configure your bar(s).

Check [here](config) for an example config file for a fully configured bar in each format.

### 3.1 Top-level options

The following table lists each of the top-level bar config options:

| Name                | Type                                    | Default | Description                                                                                                                    |
|---------------------|-----------------------------------------|---------|--------------------------------------------------------------------------------------------------------------------------------|
| `ironvar_defaults`  | `Map<string, string>`                   | `{}`    | Map of [ironvar](ironvars) keys against their default values.                                                                  |
| `monitors`          | `Map<string, BarConfig or BarConfig[]>` | `null`  | Map of monitor names against bar configs.                                                                                      |
| `icon_theme`        | `string`                                | `null`  | Name of the GTK icon theme to use. Leave blank to use default.                                                                 |
| `icon_overrides`    | `Map<string, string>`                   | `{}`    | Map of image inputs to override names. Usually used for app IDs (or classes) to icon names, overriding the app's default icon. |
| `double_click_time` | `integer` or `"gtk"`                    | `250`   | Time in milliseconds to wait for a double-click. Set to `"gtk"` to use GTK's setting.                                          |

> [!TIP]
> `monitors` is only required if you are following **2b** or **2c** (ie not the same bar across all monitors).

> [!Note]
> All bar-level options listed in the below section can also be defined at the top-level.

# 3.2 Bar-level options

The following table lists each of the bar-level bar config options:

| Name              | Type                                           | Default                                  | Description                                                                                                                |
|-------------------|------------------------------------------------|------------------------------------------|----------------------------------------------------------------------------------------------------------------------------|
| `name`            | `string`                                       | `bar-<n>`                                | A unique identifier for the bar, used for controlling it over IPC. If not set, uses a generated integer suffix.            |
| `position`        | `top` or `bottom` or `left` or `right`         | `bottom`                                 | The bar's position on screen.                                                                                              |
| `anchor_to_edges` | `boolean`                                      | `false`                                  | Whether to anchor the bar to the edges of the screen. Setting to false centres the bar.                                    |
| `height`          | `integer`                                      | `42`                                     | The bar's height in pixels.                                                                                                |
| `margin.top`      | `integer`                                      | `0`                                      | The margin on the top of the bar                                                                                           |
| `margin.bottom`   | `integer`                                      | `0`                                      | The margin on the bottom of the bar                                                                                        |
| `margin.left`     | `integer`                                      | `0`                                      | The margin on the left of the bar                                                                                          |
| `margin.right`    | `integer`                                      | `0`                                      | The margin on the right of the bar                                                                                         |
| `layer`           | `background` or `bottom` or `top` or `overlay` | `top`                                    | The layer-shell layer to place the bar on.                                                                                 |
| `exclusive_zone`  | `boolean`                                      | `true` unless `start_hidden` is enabled. | Whether the bar should reserve an exclusive zone around it.                                                                |
| `popup_gap`       | `integer`                                      | `5`                                      | The gap between the bar and popup window.                                                                                  |
| `popup_autohide`  | `boolean`                                      | `false`                                  | Whether to close the popup on outside click. On some compositors, this can aggressively steal kb/m focus.                  |
| `start_hidden`    | `boolean`                                      | `false`, or `true` if `autohide` set     | Whether the bar should be hidden when the application starts. Enabled by default when `autohide` is set.                   |
| `autohide`        | `integer`                                      | `null`                                   | The duration in milliseconds before the bar is hidden after the cursor leaves. Leave unset to disable auto-hide behaviour. |
| `start`           | `Module[]`                                     | `[]`                                     | Array of left or top modules.                                                                                              |
| `center`          | `Module[]`                                     | `[]`                                     | Array of center modules.                                                                                                   |
| `end`             | `Module[]`                                     | `[]`                                     | Array of right or bottom modules.                                                                                          |

### 3.2 Module-level options

Each module must include a `type` key.

The following table lists each of the module-level options that are present on **all** modules.
For details on available modules and each of their config options, check the sidebar.

For information on the `Script` type, and embedding scripts in strings, see [here](script).

#### Events

| Name                     | Type               | Default | Description                                                                                      |
|--------------------------|--------------------|---------|--------------------------------------------------------------------------------------------------|
| `on_click_left`          | `Script [oneshot]` | `null`  | Runs the script when the module is left clicked.                                                 |
| `on_click_left_double`   | `Script [oneshot]` | `null`  | Runs the script when the module is double-left clicked.                                          |
| `on_click_middle`        | `Script [oneshot]` | `null`  | Runs the script when the module is middle clicked.                                               |
| `on_click_middle_double` | `Script [oneshot]` | `null`  | Runs the script when the module is double-middle clicked.                                        |
| `on_click_right`         | `Script [oneshot]` | `null`  | Runs the script when the module is right clicked.                                                |
| `on_click_right_double`  | `Script [oneshot]` | `null`  | Runs the script when the module is double-right clicked.                                         |
| `on_scroll_up`           | `Script [oneshot]` | `null`  | Runs the script when the module is scroll up on.                                                 |
| `on_scroll_down`         | `Script [oneshot]` | `null`  | Runs the script when the module is scrolled down on.                                             |
| `on_mouse_enter`         | `Script [oneshot]` | `null`  | Runs the script when the module is hovered over.                                                 |
| `on_mouse_exit`          | `Script [oneshot]` | `null`  | Runs the script when the module is no longer hovered over.                                       |
| `smooth_scroll_speed`    | `float`            | `1.0`   | Speed multiplier `0.0` - `10.0` which controls scroll up/down events triggered using a trackpad. |

#### Visibility

| Name                  | Type                                                  | Default       | Description                                                                                                        |
|-----------------------|-------------------------------------------------------|---------------|--------------------------------------------------------------------------------------------------------------------|
| `show_if`             | [Dynamic Boolean](dynamic-values#dynamic-boolean)     | `null`        | Polls the script to check its exit code. If exit code is zero, the module is shown. For other codes, it is hidden. |
| `transition_type`     | `slide_start` or `slide_end` or `crossfade` or `none` | `slide_start` | The transition animation to use when showing/hiding the widget.                                                    |
| `transition_duration` | `integer`                                             | `250`         | The length of the transition animation to use when showing/hiding the widget.                                      |
| `disable_popup`       | `boolean`                                             | `false`       | Prevents the popup from opening on-click for this widget.                                                          |

#### Appearance

| Name      | Type     | Default | Description                                                                       |
|-----------|----------|---------|-----------------------------------------------------------------------------------|
| `tooltip` | `string` | `null`  | Shows this text on hover. Supports embedding scripts between `{{double braces}}`. |
| `name`    | `string` | `null`  | The unique widget name, allowing you to style it using `#name`.                   |
| `class`   | `string` | `null`  | One or more CSS classes, allowing you to style it using `.class`.                 |

For more information on styling, please see the [styling guide](styling-guide).

#### Formatting

| Name          | Type                                                   | Default                    | Description                                                                                                                                     |
|---------------|--------------------------------------------------------|----------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `orientation` | `horizontal` or `vertical` (shorthand: `'h'` or `'v'`) | `horizontal` or `vertical` | The direction in which the widget and its text are laid out. Some modules additionally provide a `direction` option to provide further control. |
| `justify`     | `left`, `right`, `center`, `fill`                      | `left`                     | The justification (alignment) of the widget text shown on the bar.                                                                              |
