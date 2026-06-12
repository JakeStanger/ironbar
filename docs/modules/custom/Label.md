Displays custom text, with markup support.

If you only intend to run a single script, prefer the [script](script) module.
For more advanced use-cases, use [custom](custom).

## Example

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

## Configuration

> Type: `label`

%{properties}%

## Styling

| Selector | Description                                                                        |
|----------|------------------------------------------------------------------------------------|
| `.label` | Label widget                                                                       |

For more information on styling, please see the [styling guide](styling-guide).