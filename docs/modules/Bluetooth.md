> [!NOTE]
> For the battery percentage option the experimental features of BlueZ needs to be enabled by adding `Experimental = true` to the `[General]` section in `/etc/bluetooth/main.conf`

Displays the current bluetooth status.
Clicking on the widget opens a popout displaying list of available devices and connection controls.

![Screenshot of bluetooth widget](https://f.jstanger.dev/github/ironbar/bluetooth.png)

## Configuration

> Type: `bluetooth`

| Name                          | Type      | Default                                               | Description                                                                                                                                      |
| ----------------------------- | --------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `format.not_found`            | `string`  | ``                                                    | Format string to use for the widget button when bluetooth adapter not found.                                                                     |
| `format.disabled`             | `string`  | ` Off`                                               | Format string to use for the widget button when bluetooth adapter is disabled.                                                                   |
| `format.enabled`              | `string`  | ` On`                                                | Format string to use for the widget button when bluetooth adapter is enabled but no devices are connected.                                       |
| `format.connected`            | `string`  | ` {device_alias}`                                    | Format string to use for the widget button when bluetooth adapter is enabled and a device is connected.                                          |
| `format.connected_battery`    | `string`  | ` {device_alias} • {device_battery_percent}%`        | Format string to use for the widget button when bluetooth adapter is enabled, a device is connected and `{device_battery_percent}` is available. |
| `popup.scrollable`            | `boolean` | `true`                                                | If true makes the popup scrollable, if false stretchable to show all of its content.                                                             |
| `popup.header`                | `string`  | ` Enable Bluetooth`                                  | Format string to use for the header of popup window.                                                                                             |
| `popup.disabled`              | `string`  | `{adapter_status}`                                    | Format string to use for the message that is displayed when the adapter is not found or disabled.                                                |
| `popup.device.header`         | `string`  | `{device_alias}`                                      | Format string to use for the header of device box.                                                                                               |
| `popup.device.header_battery` | `string`  | `{device_alias}`                                      | Format string to use for the header of device box when `{device_battery_percent}` is available.                                                  |
| `popup.device.footer`         | `string`  | `{device_status}`                                     | Format string to use for the footer of device box.                                                                                               |
| `popup.device.footer_battery` | `string`  | `{device_status} • Battery {device_battery_percent}%` | Format string to use for the footer of device box when `{device_battery_percent}` is available.                                                  |
| `adapter_status.not_found`    | `string`  | `No Bluetooth adapters found`                         | The value of `{adapter_status}` formatting token when adapter not found.                                                                         |
| `adapter_status.enabled`      | `string`  | `Bluetooth enabled`                                   | The value of `{adapter_status}` formatting token when adapter is enabled.                                                                        |
| `adapter_status.enabling`     | `string`  | `Enabling Bluetooth...`                               | The value of `{adapter_status}` formatting token when adapter is enabling.                                                                       |
| `adapter_status.disabled`     | `string`  | `Bluetooth disabled`                                  | The value of `{adapter_status}` formatting token when adapter is disabled.                                                                       |
| `adapter_status.disabling`    | `string`  | `Disabling Bluetooth...`                              | The value of `{adapter_status}` formatting token when adapter is disabling.                                                                      |
| `device_status.connected`     | `string`  | `Connected`                                           | The value of `{device_status}` formatting token when device is connected.                                                                        |
| `device_status.connecting`    | `string`  | `Connecting...`                                       | The value of `{device_status}` formatting token when device is connecting.                                                                       |
| `device_status.disconnected`  | `string`  | `Disconnect`                                          | The value of `{device_status}` formatting token when device is disconnected.                                                                     |
| `device_status.disconnecting` | `string`  | `Disconnecting...`                                    | The value of `{device_status}` formatting token when device is disconnecting.                                                                    |
| `icon_size`                   | `integer` | `32`                                                  | Size to render icon at (image icons only).                                                                                                       |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "bluetooth",
      "icon_size": 32,
      "format": {
        "not_found": "",
        "disabled": " Off",
        "enabled": " On",
        "connected": " {device_alias}",
        "connected_battery": " {device_alias} • {device_battery_percent}%"
      },
      "popup": {
        "scrollable": true,
        "header": " Enable Bluetooth",
        "disabled": "{adapter_status}",
        "device": {
          "header": "{device_alias}",
          "header_battery": "{device_alias}",
          "footer": "{device_status}",
          "footer_battery": "{device_status} • Battery {device_battery_percent}%"
        }
      },
      "adapter_status": {
        "not_found": "No Bluetooth adapters found",
        "enabled": "Bluetooth enabled",
        "enabling": "Enabling Bluetooth...",
        "disabled": "Bluetooth disabled",
        "disabling": "Disabling Bluetooth..."
      },
      "device_status": {
        "connected": "Connected",
        "connecting": "Connecting...",
        "disconnected": "Disconnect",
        "disconnecting": "Disconnecting..."
      }
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "bluetooth"
icon_size = 32

[end.format]
not_found = ""
disabled = " Off"
enabled = " On"
connected = " {device_alias}"
connected_battery = " {device_alias} • {device_battery_percent}%"

[end.popup]
scrollable = true
header = " Enable Bluetooth"
disabled = "{adapter_status}"

[end.popup.device]
header = "{device_alias}"
header_battery = "{device_alias}"
footer = "{device_status}"
footer_battery = "{device_status} • Battery {device_battery_percent}%"

[end.adapter_status]
not_found = "No Bluetooth adapters found"
enabled = "Bluetooth enabled"
enabling = "Enabling Bluetooth..."
disabled = "Bluetooth disabled"
disabling = "Disabling Bluetooth..."

[end.device_status]
connected = "Connected"
connecting = "Connecting..."
disconnected = "Disconnect"
disconnecting = "Disconnecting..."
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: bluetooth
    icon_size: 32
    format:
      not_found: ""
      disabled: " Off"
      enabled: " On"
      connected: " {device_alias}"
      connected_battery: " {device_alias} • {device_battery_percent}%"
    popup:
      scrollable: true
      header: " Enable Bluetooth"
      disabled: "{adapter_status}"
      device:
        header: "{device_alias}"
        header_battery: "{device_alias}"
        footer: "{device_status}"
        footer_battery: "{device_status} • Battery {device_battery_percent}%"
    adapter_status:
      not_found: "No Bluetooth adapters found"
      enabled: "Bluetooth enabled"
      enabling: "Enabling Bluetooth..."
      disabled: "Bluetooth disabled"
      disabling: "Disabling Bluetooth..."
    device_status:
      connected: "Connected"
      connecting: "Connecting..."
      disconnected: "Disconnect"
      disconnecting: "Disconnecting..."
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = bluetooth
      icon_size = 32
      format.not_found = ""
      format.disabled = " Off"
      format.enabled = " On"
      format.connected = " {device_alias}"
      format.connected_battery = " {device_alias} • {device_battery_percent}%"
      popup.scrollable = true
      popup.header = " Enable Bluetooth"
      popup.disabled = "{adapter_status}"
      popup.device.header = "{device_alias}"
      popup.device.header_battery = "{device_alias}"
      popup.device.footer = "{device_status}"
      popup.device.footer_battery = "{device_status} • Battery {device_battery_percent}%"
      adapter_status.not_found = "No Bluetooth adapters found"
      adapter_status.enabled = "Bluetooth enabled"
      adapter_status.enabling = "Enabling Bluetooth..."
      adapter_status.disabled = "Bluetooth disabled"
      adapter_status.disabling = "Disabling Bluetooth..."
      device_status.connected = "Connected"
      device_status.connecting = "Connecting..."
      device_status.disconnected = "Disconnect"
      device_status.disconnecting = "Disconnecting..."
    }
  ]
}
```

</details>

### Formatting Tokens

The following tokens can be used in format strings:

| Token                      | Description                                                                                                             |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `{adapter_status}`         | The current adapter status. The mapping of a status to a string could be defined using `adapter_status` config section. |
| `{device_alias}`           | The device name or address if name is not available.                                                                    |
| `{device_status}`          | The current device status. The mapping of a status to a string could be defined using `device_status` config section.   |
| `{device_battery_percent}` | The device battery percentage.                                                                                          |
| `{device_address}`         | The device address, e.g `00:11:22:33:FF:EE`.                                                                            |

## Styling

| Selector                                                       | Description                                                                          |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `.bluetooth`                                                   | Tray widget button                                                                   |
| `.bluetooth.not-found`                                         | Tray widget button when bluetooth adapter not found                                  |
| `.bluetooth.disabled`                                          | Tray widget button when bluetooth adapter is disabled                                |
| `.bluetooth.enabled`                                           | Tray widget button when bluetooth adapter is enabled but no devices are connected    |
| `.bluetooth.connected`                                         | Tray widget button when bluetooth adapter is enabled and a device is connected       |
| `.popup-bluetooth`                                             | Popup box                                                                            |
| `.popup-bluetooth .header`                                     | Header box with switch and label                                                     |
| `.popup-bluetooth .header .switch`                             | Bluetooth enable/disable switch                                                      |
| `.popup-bluetooth .header .label`                              | Bluetooth enable/disable label                                                       |
| `.popup-bluetooth .disabled`                                   | Box that is only shown in non-enabled states (e.g. disabled, adapter not found, etc) |
| `.popup-bluetooth .disabled .spinner`                          | Spinner that is only shown in "connecting" and "disconnecing" states                 |
| `.popup-bluetooth .disabled .label`                            | Label inside disabled container                                                      |
| `.popup-bluetooth .devices`                                    | Devices scrollwindow                                                                 |
| `.popup-bluetooth .devices .box`                               | Box inside devices scrollwindow                                                      |
| `.popup-bluetooth .devices .box .device`                       | Device box                                                                           |
| `.popup-bluetooth .devices .box .device .icon-box`             | Device icon box                                                                      |
| `.popup-bluetooth .devices .box .device .icon-box .icon`       | Device icon content (any type)                                                       |
| `.popup-bluetooth .devices .box .device .icon-box .text-icon`  | Device icon content (textual only)                                                   |
| `.popup-bluetooth .devices .box .device .icon-box .image`      | Device icon content (image only)                                                     |
| `.popup-bluetooth .devices .box .device .status`               | Device status box                                                                    |
| `.popup-bluetooth .devices .box .device .status .header-label` | Device header label                                                                  |
| `.popup-bluetooth .devices .box .device .status .footer-label` | Device footer label                                                                  |
| `.popup-bluetooth .devices .box .device .switch`               | Device connect/disconnect switch                                                     |
| `.popup-bluetooth .devices .box .device .spinner`              | Spinner that is only shown in "connecting" and "disconnecing" states                 |

For more information on styling, please see the [styling guide](styling-guide).
