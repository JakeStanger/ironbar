Application menu that shows installed programs and optionally custom entries. 
This works by reading all `.desktop` files on the system.

Clicking the menu button will open the main menu.
Clicking on any application category will open a sub-menu with any installed applications that match.

It is also possible to add custom categories and actions into the menu.

![Screenshot of open menu showing applications inside Office category](https://f.jstanger.dev/github/ironbar/modules/menu.png)

## Example

```corn
{
  start = [
    {
      type = "menu"
      start = [
        {
            type = "custom"
            label = "Terminal"
            on_click = "xterm"
        }
      ]
      height = 440
      width = 200
      icon = "archlinux"
      label = null
    }
  ]
}
```

## Configuration

%{properties}%

## Styling

| Selector                             | Description                       |
|--------------------------------------|-----------------------------------|
| `.menu`                              | Menu button                       |
| `.popup-menu`                        | Main container of the popup       |
| `.popup-menu .main`                  | Main menu of the menu             |
| `.popup-menu .main .category`        | Category button                   |
| `.popup-menu .main .category.open`   | Open category button              |
| `.popup-menu .main .main-start`      | Container for `start` entries     |
| `.popup-menu .main .main-center`     | Container for `center` entries    |
| `.popup-menu .main .main-end`        | Container for `end` entries       |
| `.popup-menu .sub-menu`              | All sub-menus                     |
| `.popup-menu .sub-menu .application` | Application button within submenu |

For more information on styling, please see the [styling guide](styling-guide).