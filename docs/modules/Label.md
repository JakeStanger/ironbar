Displays custom text, with markup support.

If you only intend to run a single script, prefer the [script](script) module.
For more advanced use-cases, use [custom](custom).

## Configuration

> Type: `label`

| Name                  | Type                                                 | Default | Description                                                                                                                                           |
|-----------------------|------------------------------------------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `label`               | [Dynamic String](dynamic-values#dynamic-string)      | `null`  | Text to show on label.                                                                                                                                |
| `truncate`            | `'start'` or `'middle'` or `'end'` or `off` or `Map` | `off`   | The location of the ellipses and where to truncate text from. Leave null to avoid truncating. Use the long-hand `Map` version if specifying a length. |
| `truncate.mode`       | `'start'` or `'middle'` or `'end'` or `off`          | `off`   | The location of the ellipses and where to truncate text from. Leave null to avoid truncating.                                                         |
| `truncate.length`     | `integer`                                            | `null`  | The fixed width (in chars) of the widget. Leave blank to let GTK automatically handle.                                                                |
| `truncate.max_length` | `integer`                                            | `null`  | The maximum number of characters before truncating. Leave blank to let GTK automatically handle.                                                      |


<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "label",
      "label": "random num: {{500:echo $RANDOM}}"
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "label"
label = "random num: {{500:echo $RANDOM}}"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "label"
    label: "random num: {{500:echo $RANDOM}}"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "label"
      label = "random num: {{500:echo \$RANDOM}}"
    }
  ]
}
```

</details>

## Styling

| Selector | Description                                                                        |
|----------|------------------------------------------------------------------------------------|
| `.label` | Label widget                                                                       |

For more information on styling, please see the [styling guide](styling-guide).