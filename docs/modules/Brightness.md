Brightness information about screen or led brightness levels in percent.
Allows to change the respective value via scrolling.

## Example

```corn
{
  end = [
    {
      type = "brightness"
      format = "{percentage}%"
      smooth_scroll_speed = 0.5

      mode.type = "systemd"
      mode.subsystem = "backlight"
      mode.name = "amdgpu_bl1"

      profiles.low.when = 25
      profiles.low.format = "{percentage}%"
      profiles.low.icon_label = ""

      profiles.high.when = 100
      profiles.high.format = "{percentage}%"
      profiles.high.icon_label = ""
    }
  ]
}
```

## Configuration

> Type: `brightness`

%properties%

### Default profiles

```corn
{
    level0.when = 5.0
    level0.icon_label = ""

    level10.when = 15.0
    level10.icon_label = ""
    
    level20.when = 25.0
    level20.icon_label = ""
    
    level30.when = 35.0
    level30.icon_label = ""
    
    level40.when = 45.0
    level40.icon_label = ""
    
    level50.when = 55.0
    level50.icon_label = ""
    
    level60.when = 65.0
    level60.icon_label = ""
    
    level70.when = 75.0
    level70.icon_label = ""
    
    level80.when = 85.0
    level80.icon_label = ""

    level90.when = 95.0
    level90.icon_label = ""
    
    level100.when = 100.0
    level100.icon_label = ""
}
```

## Styling

| Selector              | Description                                |
|-----------------------|--------------------------------------------|
| `.brightness`         | Brightness widget button                   |
| `.brightness .label`  | text, which is controlled via `format`     |
| `.brightness .icon`   | icon, which is controlled via `icon_label` |

For more information on styling, please see the [styling guide](/guides/styling-guide).
