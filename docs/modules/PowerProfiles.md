Power Profiles status information with icons by default.
Displays the currently active device power profile and any profile status information. 
Clicking the module cycles between available profiles.

> [!NOTE]
> This module requires that `upower` is installed and its service is running.

## Example

```corn
{
  end = [
    {
      type = "power_profiles"
      format = "{name}"

      profiles.power_saving.when = "power-saver"
      profiles.power_saving.icon_label = ""
    }
  ]
}
```

## Configuration

> Type: `power_profiles`

%properties%

### Icons

The icon is configured per profile using the `icon` option.

Power Profiles ships with default profiles that set icons for common profiles:

| Profile      | Threshold (==) | Icon |
|--------------|----------------|------|
| `power-saver`| `power-saver`  | `` |
| `balanced`   | `balanced`     | `` |
| `performance`| `performance`  | `󱐋` |

### Default profiles

<details>
<summary>Show</summary>

```corn
{
    power-saver.when = "power-saver"
    power-saver.icon = ""

    balanced.when = "balanced"
    balanced.icon = ""
    
    performance.when = "performance"
    performance.icon = "󱐋"
}
```

</details>

## Styling

| Selector                               | Description                                     |
|----------------------------------------|-------------------------------------------------|
| `.power_profiles`                      | Power Profiles widget button                    |
| `.power_profiles .label`               | text, which is controlled via `format`          |
| `.power_profiles .tooltip`             | text, which is controlled via `tooltip_format`  |
| `.power_profiles .icon`                | icon, which is controlled via `icon`            |
| `.power_profiles .performance_degraded`| indicates that performance is currently degraded|

For more information on styling, please see the [styling guide](styling-guide).
