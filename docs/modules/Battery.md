> [!NOTE]
> This module requires that `upower` is installed and its service running.

Displays system power information such as the battery percentage, and estimated time to empty.

`TODO: ADD SCREENSHOT`

## Example

```corn
{
  end = [
    {
      type = "battery"
      format = "{percentage}%"
      profiles.warning = 20
      
      profiles.critical.when = { percent = 5 charging = false }
      profiles.critical.format = "[LOW] {percentage}%"
    }
  ]
}
```

## Configuration

> Type: `battery`

%{properties}%

## Styling

| Selector                  | Description                                     |
|---------------------------|-------------------------------------------------|
| `.battery`                | Battery widget button.                          |
| `.battery.<threshold>`    | Battery widget button (dynamic threshold class) |
| `.battery .contents`      | Battery widget button contents.                 |
| `.battery .icon`          | Battery widget battery icon.                    |
| `.battery .label`         | Battery widget button label.                    |
| `.popup-battery`          | Battery popup box.                              |
| `.popup-battery .details` | Label inside the popup.                         |

For more information on styling, please see the [styling guide](styling-guide).
