> [!TIP]
> Inhibit is tracked globally (system-wide).
> Multiple inhibit modules share the same inhibit state.

Prevents the compositor from auto-suspending the system or auto-locking the screen. Click to toggle inhibit on/off or cycle through preset durations.



## Example

```corn
{
  end = [
    {
      type = "inhibit"
      durations = ["00:30:00" "01:00:00" "01:30:00" "02:00:00" "inf"]
      default_duration = "00:30:00"
      on_click_left = "toggle"
      on_click_right = "cycle"
      format_on = "☕ {duration}"
      format_off = "💤 {duration}"
    }
  ]
}
```

## Configuration

> Type: `inhibit`

%{properties}%

## Styling

| Selector   | Description           |
|------------|-----------------------|
| `.inhibit` | Inhibit widget button |

For more information on styling, please see the [styling guide](styling-guide).
