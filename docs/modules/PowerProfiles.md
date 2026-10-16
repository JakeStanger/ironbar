Power Profiles status information with icons by default.
You can switch profiles by clicking.

## Configuration

> Type: `power_profiles`

| Name                   | Type                    | Default                | Profile? | Description                                                      |
|------------------------|-------------------------|------------------------|----------|------------------------------------------------------------------|
| `format`               | `string`                | `null`                 | No       |Format string to use for the widget button label.                 |
| `tooltip_format`       | `string`                | `Power profile: {name}`| No       |Format string to use for the widget tooltip. Leave empty for no tooltip.                 |
| `icon`                 | `string`                | `null`                 | Yes      | Icon to show alongside the label. Supports [image](images) icons.|
| `use_default_profiles` | `boolean`               | `true`                 | No       | Whether default profiles should be used.                         |

Information on the profiles system can be found [here](profiles).

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "power_profiles",
      "format": "{name}",
      "profiles": {
        "power-saving": {
          "when": "power-saver",
          "icon": ""
        },
      }
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "power_profiles"
format = "{name}"

[end.profiles.power_saving]
when = "power-saver"
icon = ""

```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: power_profiles
    format: "{name}"

    profiles:
      power_saving:
        when: "power-saver"
        icon: ""
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "power_profile"
      format = "{name}"

      profiles.power_saving.when = "power-saver"
      profiles.power_saving.icon_label = ""
    }
  ]
}
```

</details>

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

### Formatting Tokens

The following tokens can be used in the `format` config option:

| Token          | Description                            |
|----------------|----------------------------------------|
| `{name}`       | Name of the active profile.            |

## Styling

| Selector                               | Description                                     |
|----------------------------------------|-------------------------------------------------|
| `.power_profiles`                      | Power Profiles widget button                    |
| `.power_profiles .label`               | text, which is controlled via `format`          |
| `.power_profiles .tooltip`             | text, which is controlled via `tooltip_format`  |
| `.power_profiles .icon`                | icon, which is controlled via `icon`            |
| `.power_profiles .performance_degraded`| indicates that performance is currently degraded|

For more information on styling, please see the [styling guide](styling-guide).
