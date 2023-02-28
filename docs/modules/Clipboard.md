Shows recent clipboard items, allowing you to switch between them to re-copy previous values.
Clicking the icon button opens the popup containing all functionality.

Supports plain text and images.

![Screenshot of clipboard popup open, with two textual values and an image copied. Several other unrelated widgets are visible on the bar.](https://f.jstanger.dev/github/ironbar/clipboard.png?raw)

## Configuration

> Type: `clipboard`

| Name                  | Type                                  | Default | Description                                                                                                                                           |
|-----------------------|---------------------------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `icon`                | `string/image`                        | `󰨸`    | Icon to show on the widget button.                                                                                                                    |
| `max_items`           | `integer`                             | `10`    | Maximum number of items to show on the bar.                                                                                                           |
| `truncate`            | `start` or `middle` or `end` or `Map` | `null`  | The location of the ellipses and where to truncate text from. Leave null to avoid truncating. Use the long-hand `Map` version if specifying a length. |
| `truncate.mode`       | `start` or `middle` or `end`          | `null`  | The location of the ellipses and where to truncate text from. Leave null to avoid truncating.                                                         |
| `truncate.length`     | `integer`                             | `null`  | The fixed width (in chars) of the widget. Leave blank to let GTK automatically handle.                                                                |
| `truncate.max_length` | `integer`                             | `null`  | The maximum number of characters before truncating. Leave blank to let GTK automatically handle.                                                      |

See [here](images) for information on images.

<details>
<summary>JSON</summary>

```json
{
  "end": {
    "type": "clipboard",
    "max_items": 3,
    "truncate": {
      "mode": "end",
      "length": 50
    }
  }
}
```
</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "clipboard"
max_items = 3

[[end.truncate]]
mode = "end"
length = 50
```
</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: 'clipboard'
    max_items: 3
    truncate:
      mode: 'end'
      length: 50
```
</details>

<details>
<summary>Corn</summary>

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
</details>

## Styling

| Selector                             | Description                                          |
|--------------------------------------|------------------------------------------------------|
| `#clipboard`                         | Clipboard widget.                                    |
| `#clipboard .btn`                    | Clipboard widget button.                             |
| `#popup-clipboard`                   | Clipboard popup box.                                 |
| `#popup-clipboard .item`             | Clipboard row item inside the popup.                 |
| `#popup-clipboard .item .btn`        | Clipboard row item radio button.                     |
| `#popup-clipboard .item .btn.text`   | Clipboard row item radio button (text values only).  |
| `#popup-clipboard .item .btn.image`  | Clipboard row item radio button (image values only). |
| `#popup-clipboard .item .btn-remove` | Clipboard row item remove button.                    |