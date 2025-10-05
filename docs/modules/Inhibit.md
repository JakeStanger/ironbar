Prevents the system from going to sleep or idle. Click to toggle inhibit on/off or cycle through preset durations.

Supports systemd and Wayland idle inhibit protocols.

## Configuration

> Type: `inhibit`

| Name              | Type                       | Default         | Description                                                                                 |
| ----------------- | -------------------------- | --------------- | ------------------------------------------------------------------------------------------- |
| `backend`         | `'systemd'` or `'wayland'` | `'systemd'`     | Backend to use for inhibiting idle/sleep.                                                   |
| `durations`       | `string[]`                 | See below       | List of durations to cycle through. Prefix with `*` to set as default. See duration format. |
| `on_click_left`   | `'toggle'` or `'cycle'`    | `'toggle'`      | Action on left click.                                                                       |
| `on_click_right`  | `'toggle'` or `'cycle'`    | `'cycle'`       | Action on right click.                                                                      |
| `on_click_middle` | `'toggle'` or `'cycle'`    | `null`          | Action on middle click.                                                                     |
| `format_on`       | `string`                   | `☕ {duration}` | Format string when inhibit is active. Pango markup supported.                               |
| `format_off`      | `string`                   | `💤 {duration}` | Format string when inhibit is inactive. Pango markup supported.                             |

> **Note:** The systemd backend persists across ironbar restarts; wayland does not.

**Default durations:** `["30m", "1h", "1h30m", "*2h", "inf"]`

**Duration format:** Accepts human-readable durations like `30m`, `1h30m`, `2h`. Use `inf`, `infinity`, `∞`, or empty string for infinite duration. Prefix with `*` to set as default.

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
      "backend": "systemd",
      "durations": ["30m", "1h", "1h30m", "*2h", "inf"],
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
backend = "systemd"
durations = ["30m", "1h", "1h30m", "*2h", "inf"]
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
    backend: "systemd"
    durations: ["30m", "1h", "1h30m", "*2h", "inf"]
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
      backend = "systemd"
      durations = ["30m" "1h" "1h30m" "*2h" "inf"]
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
