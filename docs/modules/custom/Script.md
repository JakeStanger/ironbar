Executes a script and shows the result of `stdout` on a label.
Pango markup is supported.

If you want to be able to embed multiple scripts and/or variables, prefer the [label](/modules/custom/label) module.
For more advanced use-cases, use [custom](/modules/custom/custom).

## Example

```corn
{
  end = [
    {
      type = "script"
      cmd = "/home/jake/.local/bin/phone-battery"
      mode = "poll"
      interval = 5000
    }
  ]
}
```

## Configuration

> Type: `script`

%properties%


## Styling

| Selector  | Description         |
|-----------|---------------------|
| `.script` | Script widget label |

For more information on styling, please see the [styling guide](/guides/styling-guide).
