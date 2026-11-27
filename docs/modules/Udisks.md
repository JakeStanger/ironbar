Displays removable drives (USB sticks, external disks, card readers) and lets you open or safely remove them.
Clicking on the widget opens a popout listing the connected drives.
Clicking a drive opens it in your default file manager, mounting it first if needed.
Clicking the eject button next to a drive unmounts its volumes and ejects it.

## Requirements

- [UDisks2](https://storaged.org/doc/udisks2-api/latest/) (`udisksd`) must be installed and running.
- Mounting and ejecting go through polkit. Ironbar never opens an authentication prompt, so if polkit requires credentials for the operation, it fails instead.
- Opening a drive requires a file manager that implements the `org.freedesktop.FileManager1` D-Bus interface (Nautilus, Dolphin, Thunar, Nemo, and others).

## Configuration

> Type: `udisks`

| Name                     | Type      | Default | Description                                                              |
| ------------------------ | --------- | ------- | ------------------------------------------------------------------------ |
| `icon`                   | `string`  | `🖴`    | The icon to show on the bar widget button. Supports [image](images) icons. |
| `icon_size`              | `integer` | `32`    | The size to render the bar icon at, in pixels. Image icons only.         |
| `eject_button_icon`      | `string`  | `⏏`    | The icon to show on each drive's eject button. Supports [image](images) icons. |
| `eject_button_icon_size` | `integer` | `32`    | The size to render the eject button icon at. Image icons only.           |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "udisks",
      "icon": "🖴",
      "icon_size": 32,
      "eject_button_icon": "⏏",
      "eject_button_icon_size": 32
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "udisks"
icon = "🖴"
icon_size = 32
eject_button_icon = "⏏"
eject_button_icon_size = 32
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: udisks
    icon: "🖴"
    icon_size: 32
    eject_button_icon: "⏏"
    eject_button_icon_size: 32
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "udisks"
      icon = "🖴"
      icon_size = 32
      eject_button_icon = "⏏"
      eject_button_icon_size = 32
    }
  ]
}
```

</details>

## Behaviour

- Only drives that UDisks2 reports as removable are listed. Some USB disks report themselves as non-removable and will not appear.
- Each entry shows the drive model and its size. The size is omitted when it is unknown, for example on a card reader with no card inserted.
- **Open:** every filesystem on the drive is opened in the default file manager. Filesystems that are not mounted are mounted first.
- **Eject:** every mounted filesystem on the drive is unmounted, then the drive is ejected and powered off when the hardware supports it. If any volume is busy, the operation fails and the drive stays connected.
- The list updates live as drives are connected and disconnected.

## Styling

| Selector                              | Description                                   |
| ------------------------------------- | --------------------------------------------- |
| `.udisks`                             | Bar widget button                             |
| `.popup-udisks`                       | Popup box                                     |
| `.popup-udisks .drive`                | Row for a single drive                        |
| `.popup-udisks .drive .drive-label`   | Button showing the drive name, opens the drive |
| `.popup-udisks .drive .eject-button`  | Eject button                                  |

For more information on styling, please see the [styling guide](styling-guide).
