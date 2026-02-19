> [!NOTE]
> This module requires the [wlr data control](https://wayland.app/protocols/wlr-data-control-unstable-v1) protocol.

Shows recent clipboard items, allowing you to switch between them to re-copy previous values.
Clicking the icon button opens the popup containing all functionality.

Supports plain text and images.

![Screenshot of clipboard popup open, with two textual values and an image copied. Several other unrelated widgets are visible on the bar.](https://f.jstanger.dev/github/ironbar/modules/clipboard.png)

## Example

```corn
{
    end = [ { 
        type = "clipboard" 
        max_items = 3 
        truncate.mode = "end" 
        truncate.length = 50 
    } ] 
}
```

## Configuration

> Type: `clipboard`

%{properties}%

## Styling

| Selector                             | Description                                          |
|--------------------------------------|------------------------------------------------------|
| `.clipboard`                         | Clipboard widget.                                    |
| `.clipboard .btn`                    | Clipboard widget button.                             |
| `.clipboard .btn .icon`              | Clipboard widget button icon (any type).             |
| `.clipboard .btn .text-icon`         | Clipboard widget button icon (textual only).         |
| `.clipboard .btn .image`             | Clipboard widget button icon (image only).           |
| `.popup-clipboard`                   | Clipboard popup box.                                 |
| `.popup-clipboard .item`             | Clipboard row item inside the popup.                 |
| `.popup-clipboard .item .btn`        | Clipboard row item radio button.                     |
| `.popup-clipboard .item .btn.text`   | Clipboard row item radio button (text values only).  |
| `.popup-clipboard .item .btn.image`  | Clipboard row item radio button (image values only). |
| `.popup-clipboard .item .btn-remove` | Clipboard row item remove button.                    |

For more information on styling, please see the [styling guide](styling-guide).