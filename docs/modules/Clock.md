Displays the current date and time. 
Clicking on the widget opens a popup with the time and a calendar.

![Screenshot of clock widget with popup open](https://user-images.githubusercontent.com/5057870/184540521-2278bdec-9742-46f0-9ac2-58a7b6f6ea1d.png)


## Configuration

> Type: `clock`

| Name           | Type     | Default                            | Description                                                                         |
|----------------|----------|------------------------------------|-------------------------------------------------------------------------------------|
| `format`       | `string` | `%d/%m/%Y %H:%M`                   | Date/time format string. Pango markup is supported.                                 |
| `format_popup` | `string` | `%H:%M:%S`                         | Date/time format string to display in the popup header. Pango markup is supported.  |
| `locale`       | `string` | `$LC_TIME` or `$LANG` or `'POSIX'` | Locale to use (eg `en_GB`). Defaults to the system language (reading from env var). |
| `orientation` | `'horizontal'` or `'vertical'` (shorthand: `'h'` or `'v'`) | `'horizontal'` | Orientation of the time on the clock button.                                                                                                      |

> Detail on available tokens can be found here: <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "clock",
      "format": "%d/%m/%Y %H:%M"
    }
  ]
}

```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "clock"
format = "%d/%m/%Y %H:%M"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "clock"
    format: "%d/%m/%Y %H:%M"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "clock"
      format = "%d/%m/%Y %H:%M"
    }
  ]
}
```

</details>

## Styling

| Selector                       | Description                                                                        |
|--------------------------------|------------------------------------------------------------------------------------|
| `.clock`                       | Clock widget button                                                                |
| `.popup-clock`                 | Clock popup box                                                                    |
| `.popup-clock .calendar-clock` | Clock inside the popup                                                             |
| `.popup-clock .calendar`       | Calendar widget inside the popup. GTK provides some OOTB styling options for this. |

For more information on styling, please see the [styling guide](styling-guide).
