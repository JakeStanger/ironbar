Displays the state of each network device managed by NetworkManager. Each device
type will show an icon representing its current state (connected, acquiring,
disconnected).

## Configuration

> Type: `network_manager`

| Name                  | Type       | Default                       | Profile? | Description                                                                    |
| --------------------- | ---------- | ----------------------------- | -------- | ------------------------------------------------------------------------------ |
| `icon_size`           | `integer`  | `24`                          | No       | Size to render icon at.                                                        |
| `types_blacklist`     | `string[]` | `[]`                          | No       | Any device with a type in this list will not be shown.                         |
| `types_whitelist`     | `string[]` | `[]`                          | No       | If not empty, only devices with a type in this list will be shown.             |
| `interface_blacklist` | `string[]` | `[]`                          | No       | Any device whose interface name is in this list will not be shown.             |
| `interface_whitelist` | `string[]` | `[]`                          | No       | If not empty, only devices whose interface name is in this list will be shown. |
| `icon`                | `string`   | `icon:network-wired-symbolic` | Yes      | Icon for connected wired device.                                               |

Information on the profiles system can be found [here](profiles).

**Device Types:** The device types used in `types_whitelist` and
`types_blacklists` are the same as those used by NetworkManager. You can find
the type of the devices on your system by running `nmcli device status` in a
terminal. The possible device types are: `unknown`, `ethernet`, `wifi`, `bt`,
`olpc_mesh`, `wimax`, `modem`, `infiniband`, `bond`, `vlan`, `adsl`, `bridge`,
`generic`, `team`, `tun`, `ip_tunnel`, `macvlan`, `vxlan`, `veth`, `macsec`,
`dummy`, `ppp`, `ovs_interface`, `ovs_port`, `ovs_bridge`, `wpan`, `six_lowpan`,
`wireguard`, `wifi_p2p`, `vrf`, `loopback`, `hsr` and `ipvlan`.

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "network_manager",
      "icon_size": 24,
      "types_blacklist": ["loopback", "bridge"],
      "profiles": {
        "wired_disconnected": {
          "when": { "type": "Wired", "state": "Disconnected" },
          "icon": "icon:network-wired-disconnected-symbolic"
        }
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
type = "network_manager"
icon_size = 24
types_blacklist = ["loopback", "bridge"]

[end.profiles.wired_disconnected]
when = { type = "Wired", state = "Disconnected" }
icon = "icon:network-wired-disconnected-symbolic"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "network_manager"
    icon_size: 24
    types_blacklist:
      - loopback
      - bridge
    profiles:
      wired_disconnected:
        when:
          type: "Wired"
          state: "Disconnected"
        icon: "icon:network-wired-disconnected-symbolic"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "network_manager"
      icon_size = 24
      types_blacklist = [ "loopback" "bridge" ]
      profiles = {
        wired_disconnected = {
          when = { type = "Wired" state = "Disconnected" }
          icon = "icon:network-wired-disconnected-symbolic"
        }
      }
    }
  ]
}
```

</details>

<details>
<summary> <b>Default profiles:</b> </summary>

```corn
profiles = {
wired_disconnected = {
  when = { type = "Wired" state = "Disconnected" }
  icon = ""
}
wired_acquiring = {
  when = { type = "Wired" state = "Acquiring" }
  icon = "icon:network-wired-acquiring-symbolic"
}
wired_connected = {
  when = { type = "Wired" state = "Connected" }
  icon = "icon:network-wired-symbolic"
}
wifi_disconnected = {
  when = { type = "Wifi" state = "Disconnected" }
  icon = ""
}
wifi_acquiring = {
  when = { type = "Wifi" state = "Acquiring" }
  icon = "icon:network-wireless-acquiring-symbolic"
}
wifi_connected_none = {
  when = { type = "Wifi" state = "Connected" signal_strength = 20 }
  icon = "icon:network-wireless-signal-none-symbolic"
}
wifi_connected_weak = {
  when = { type = "Wifi" state = "Connected" signal_strength = 40 }
  icon = "icon:network-wireless-signal-weak-symbolic"
}
wifi_connected_ok = {
  when = { type = "Wifi" state = "Connected" signal_strength = 50 }
  icon = "icon:network-wireless-signal-ok-symbolic"
}
wifi_connected_good = {
  when = { type = "Wifi" state = "Connected" signal_strength = 80 }
  icon = "icon:network-wireless-signal-good-symbolic"
}
wifi_connected_excellent = {
  when = { type = "Wifi" state = "Connected" signal_strength = 100 }
  icon = "icon:network-wireless-signal-excellent-symbolic"
}
cellular_disconnected = {
  when = { type = "Cellular" state = "Disconnected" }
  icon = ""
}
cellular_acquiring = {
  when = { type = "Cellular" state = "Acquiring" }
  icon = "icon:network-cellular-acquiring-symbolic"
}
cellular_connected = {
  when = { type = "Cellular" state = "Connected" }
  icon = "icon:network-cellular-connected-symbolic"
}
vpn_disconnected = {
  when = { type = "Vpn" state = "Disconnected" }
  icon = ""
}
vpn_acquiring = {
  when = { type = "Vpn" state = "Acquiring" }
  icon = "icon:network-vpn-acquiring-symbolic"
}
vpn_connected = {
  when = { type = "Vpn" state = "Connected" }
  icon = "icon:network-vpn-symbolic"
}
unknown = {
  when = { type = "Unknown" }
  icon = "icon:dialog-question-symbolic"
}
}
```

</details>


## Styling

| Selector                 | Description                      |
| ------------------------ | -------------------------------- |
| `.network_manager`       | NetworkManager widget container. |
| `.network_manager .icon` | NetworkManager widget icons.     |

For more information on styling, please see the [styling guide](styling-guide).
