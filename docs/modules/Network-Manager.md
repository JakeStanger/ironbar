Displays the state of each network device managed by NetworkManager. Each device
type will show an icon representing its current state (connected, acquiring,
disconnected).

## Configuration

> Type: `networkmanager`

| Name                          | Type       | Default                                    | Description                                                                         |
| ----------------------------- | ---------- | ------------------------------------------ | ----------------------------------------------------------------------------------- |
| `icon_size`                   | `integer`  | `24`                                       | Size to render icon at.                                                             |
| `types_blacklist`             | `string[]` | `[]`                                       | Any device with a type in this list will not be shown.                              |
| `types_whitelist`             | `string[]` | `[]`                                       | If not empty, only devices with a type in this list will be shown.                  |
| `interface_blacklist`         | `string[]` | `[]`                                       | Any device whose interface name is in this list will not be shown.                  |
| `interface_whitelist`         | `string[]` | `[]`                                       | If not empty, only devices whose interface name is in this list will be shown.      |
| `icons.wired.connected`       | `string`   | `icon:network-wired-symbolic`              | Icon for connected wired device.                                                    |
| `icons.wired.acquiring`       | `string`   | `icon:network-wired-acquiring-symbolic`    | Icon for acquiring wired device.                                                    |
| `icons.wired.disconnected`    | `string`   | `""`                                       | Icon for disconnected wired device.                                                 |
| `icons.wifi.levels`           | `string[]` | See below                                  | Icon for each strengh level of a connected wifi connection, from lowest to highest. |
| `icons.wifi.acquiring`        | `string`   | `icon:network-wireless-acquiring-symbolic` | Icon for acquiring wifi device.                                                     |
| `icons.wifi.disconnected`     | `string`   | `""`                                       | Icon for disconnected wifi connection.                                              |
| `icons.cellular.connected`    | `string`   | `icon:network-cellular-connected-symbolic` | Icon for connected cellular device.                                                 |
| `icons.cellular.acquiring`    | `string`   | `icon:network-cellular-acquiring-symbolic` | Icon for acquiring cellular device.                                                 |
| `icons.cellular.disconnected` | `string`   | `""`                                       | Icon for disconnected cellular device.                                              |
| `icons.vpn.connected`         | `string`   | `icon:network-vpn-symbolic`                | Icon for connected VPN device.                                                      |
| `icons.vpn.acquiring`         | `string`   | `icon:network-vpn-acquiring-symbolic`      | Icon for acquiring VPN device.                                                      |
| `icons.vpn.disconnected`      | `string`   | `""`                                       | Icon for disconnected VPN device.                                                   |
| `unkown`                      | `string`   | `icon:dialog-question-symbolic`            | Icon for device in unkown state.                                                    |

**Default `icons.wifi.levels`:** they contain the 5 GTK symbolic icons for wireless signal strength:
- `"icon:network-wireless-signal-none-symbolic"`
- `"icon:network-wireless-signal-weak-symbolic"`
- `"icon:network-wireless-signal-ok-symbolic"`
- `"icon:network-wireless-signal-good-symbolic"`
- `"icon:network-wireless-signal-excellent-symbolic"`

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "networkmanager",
      "icon_size": 24
      types_blacklist: ["loopback", "bridge"]
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "networkmanager"
icon_size = 24
types_blacklist = ["loopback", "bridge"]
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "networkmanager"
    icon_size: 24
    types_blacklist:
      - loopback
      - bridge
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "networkmanager"
      icon_size = 24
      types_blacklist = ["loopback", "bridge"]
    }
  ]
}
```

</details>

## Styling

| Selector               | Description                      |
| ---------------------- | -------------------------------- |
| `.networkmanager`      | NetworkManager widget container. |
| `.networkmanger .icon` | NetworkManager widget icons.     |

For more information on styling, please see the [styling guide](styling-guide).
