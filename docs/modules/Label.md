Displays custom text, with markup support.

If you only intend to run a single script, prefer the [script](script) module.
For more advanced use-cases, use [custom](custom).

## Configuration

> Type: `label`

| Name    | Type                                            | Default | Description            |
|---------|-------------------------------------------------|---------|------------------------|
| `label` | [Dynamic String](dynamic-values#dynamic-string) | `null`  | Text to show on label. |

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
      label = "random num: {{500:echo $RANDOM}}"
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