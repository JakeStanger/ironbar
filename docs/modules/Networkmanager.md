Displays the current network connection state of NetworkManager.
Supports wired ethernet, wifi, cellular data and VPN connections among others.

> [!NOTE]
> This module uses NetworkManager's so-called primary connection, and therefore inherits its limitation of only being able to display the "top-level" connection.
> For example, if we have a VPN connection over a wifi connection it will only display the former, until it is disconnected, at which point it will display the latter.
> A solution to this is currently in the works.

## Configuration

> Type: `networkmanager`

| Name                          | Type       | Default                                               | Description                                                                                                        |
| ----------------------------- | ---------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `icon_size`                   | `integer`  | `24`                                                  | Size to render icon at.                                                                                            |
| `icons.wired.connected`       | `string`   | `icon:network-wired-symbolic`                         | Icon for connected wired device                                                                                    |
| `icons.wired.acquiring`       | `string`   | `icon:network-wired-acquiring-symbolic`               | Icon for acquiring wired device                                                                                    |
| `icons.wired.disconnected`    | `string`   | `""`                                                  | Icon for disconnected wired device                                                                                 |
| `icons.wifi.levels`           | `string[]` | `["icon:network-wireless-signal-none-symbolic", ...]` | Icon for each strengh level of a connected wifi connection, from lowest to highest. The default contains 5 levels. |
| `icons.wifi.acquiring`        | `string`   | `icon:network-wireless-acquiring-symbolic`            | Icon for acquiring wifi device                                                                                     |
| `icons.wifi.disconnected`     | `string`   | `""`                                                  | Icon for disconnected wifi connection                                                                              |
| `icons.cellular.connected`    | `string`   | `icon:network-cellular-connected-symbolic`            | Icon for connected cellular device                                                                                 |
| `icons.cellular.acquiring`    | `string`   | `icon:network-cellular-acquiring-symbolic`            | Icon for acquiring cellular device                                                                                 |
| `icons.cellular.disconnected` | `string`   | `""`                                                  | Icon for disconnected cellular device                                                                              |
| `icons.vpn.connected`         | `string`   | `icon:network-vpn-symbolic`                           | Icon for connected VPN device                                                                                      |
| `icons.vpn.acquiring`         | `string`   | `icon:network-vpn-acquiring-symbolic`                 | Icon for acquiring VPN device                                                                                      |
| `icons.vpn.disconnected`      | `string`   | `""`                                                  | Icon for disconnected VPN device                                                                                   |
| `unkown`                      | `string`   | `icon:dialog-question-symbolic`                       | Icon for device in unkown state                                                                                    |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "networkmanager",
      "icon_size": 32
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
icon_size = 32
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "networkmanager"
    icon_size: 32
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "networkmanager"
      icon_size = 32
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
