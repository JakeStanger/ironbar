> [!NOTE]
> For the battery percentage option the experimental features of BlueZ needs to be enabled by adding `Experimental = true` to the `[General]` section in `/etc/bluetooth/main.conf`

Displays the current bluetooth status.
Clicking on the widget opens a popout displaying list of available devices and connection controls.

![Screenshot of bluetooth widget](https://f.jstanger.dev/github/ironbar/bluetooth.png)

## Example

```corn
{
  end = [
    {
      type = "bluetooth"
      icon_size = 32
      format.not_found = ""
      format.disabled = " Off"
      format.enabled = " On"
      format.connected = " {device_alias}"
      format.connected_battery = " {device_alias} • {device_battery_percent}%"
      popup.max_height.devices = 5
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

## Configuration

> Type: `bluetooth`

%{properties}%

## Styling

| Selector                                                       | Description                                                                          |
|----------------------------------------------------------------|--------------------------------------------------------------------------------------|
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
