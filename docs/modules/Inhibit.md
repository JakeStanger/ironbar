Prevents the system from going to sleep or idle using Wayland idle inhibit protocol. Click to toggle inhibit on/off or cycle through preset durations.

## Configuration

> Type: `inhibit`

| Name               | Type                    | Default    | Description                                                                       |
| ------------------ | ----------------------- | ---------- | --------------------------------------------------------------------------------- |
| `durations`        | `string[]`              | See below  | List of durations to cycle through. See duration format.                          |
| `default_duration` | `string`                | `02:00:00` | The default duration to use when starting inhibit. Picks first item if unmatched. |
| `on_click_left`    | `'toggle'` or `'cycle'`    | `'toggle'`      | Action on left click.                                                             |
| `on_click_right`   | `'toggle'` or `'cycle'`    | `'cycle'`       | Action on right click.                                                            |
| `on_click_middle`  | `'toggle'` or `'cycle'`    | `null`          | Action on middle click.                                                           |
| `format_on`        | `string`                | `☕ {duration}` | Format string when inhibit is active. Pango markup supported.                     |
| `format_off`       | `string`                | `💤 {duration}` | Format string when inhibit is inactive. Pango markup supported.                   |

**Default durations:** `["00:30:00", "01:00:00", "01:30:00", "02:00:00", "0"]`

**Duration format:** Time format `HH:MM:SS` (e.g., `01:30:00` for 1 hour 30 minutes). Use `0` for infinite duration.

### Click Actions

- **`toggle`**: Toggles inhibit on/off using the selected duration
- **`cycle`**: Cycles to the next duration in the list (and applies it if already active)

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "inhibit",
      "durations": ["00:30:00", "01:00:00", "01:30:00", "02:00:00", "0"],
      "default_duration": "02:00:00",
      "on_click_left": "toggle",
      "on_click_right": "cycle",
      "format_on": "☕ {duration}",
      "format_off": "💤 {duration}"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "inhibit"
durations = ["00:30:00", "01:00:00", "01:30:00", "02:00:00", "0"]
default_duration = "02:00:00"
on_click_left = "toggle"
on_click_right = "cycle"
format_on = "☕ {duration}"
format_off = "💤 {duration}"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "inhibit"
    durations: ["00:30:00", "01:00:00", "01:30:00", "02:00:00", "0"]
    default_duration: "02:00:00"
    on_click_left: "toggle"
    on_click_right: "cycle"
    format_on: "☕ {duration}"
    format_off: "💤 {duration}"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "inhibit"
      durations = ["00:30:00" "01:00:00" "01:30:00" "02:00:00" "0"]
      default_duration = "02:00:00"
      on_click_left = "toggle"
      on_click_right = "cycle"
      format_on = "☕ {duration}"
      format_off = "💤 {duration}"
    }
  ]
}
```

</details>

### Formatting Tokens

| Token        | Description                                          |
| ------------ | ---------------------------------------------------- |
| `{duration}` | Current duration (formatted as human-readable time). |

## Styling

| Selector   | Description           |
| ---------- | --------------------- |
| `.inhibit` | Inhibit widget button |

For more information on styling, please see the [styling guide](styling-guide).
