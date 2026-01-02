Displays the current date and time. 
Clicking on the widget opens a popup with the time and a calendar.

![Screenshot of clock widget with popup open](https://f.jstanger.dev/github/ironbar/modules/clock.png)


## Example

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

## Configuration

> Type: `clock`

%{properties}%

## Styling

| Selector                        | Description                                                                        |
|---------------------------------|------------------------------------------------------------------------------------|
| `.clock`                        | Clock widget button                                                                |
| `.popup-clock`                  | Clock popup box                                                                    |
| `.popup-clock .calendar-clock`  | Clock inside the popup                                                             |
| `.popup-clock .calendar`        | Calendar widget inside the popup. GTK provides some OOTB styling options for this. |

Information on styling the calendar can be found [here](https://docs.gtk.org/gtk4/class.Calendar.html#css-nodes).

For more information on styling, please see the [styling guide](styling-guide).
