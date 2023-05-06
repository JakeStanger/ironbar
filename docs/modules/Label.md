Displays custom text, with the ability to embed [scripts](https://github.com/JakeStanger/ironbar/wiki/scripts#embedding).

## Configuration

> Type: `label`

| Name    | Type     | Default | Description                             |
|---------|----------|---------|-----------------------------------------|
| `label` | `string` | `null`  | Text, optionally with embedded scripts. |

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