Displays the state of each network device managed by NetworkManager. Each device
type will show an icon representing its current state (connected, acquiring,
disconnected).

## Example

```corn
{
  end = [
    {
      type = "network_manager"
      icon_size = 24
      types_blacklist = [ "loopback" "bridge" ]
      profiles = {
        wired_disconnected = {
          when = { type = "wired" state = "disconnected" }
          icon = "icon:network-wired-disconnected-symbolic"
        }
      }
    }
  ]
}
```


## Configuration

> Type: `network_manager`

> [!NOTE]
> This module does not support module-level [layout options](module-level-options#layout).


%{properties}%

---

<details>
<summary> <b>Default profiles:</b> </summary>

```corn
{
    profiles = {
        wired_disconnected = {
            when = { type = "wired" state = "disconnected" }
            icon = ""
        }
        wired_acquiring = {
            when = { type = "wired" state = "acquiring" }
            icon = "icon:network-wired-acquiring-symbolic"
        }
        wired_connected = {
            when = { type = "wired" state = "connected" }
            icon = "icon:network-wired-symbolic"
        }
        wifi_disconnected = {
            when = { type = "wifi" state = "disconnected" }
            icon = ""
        }
        wifi_acquiring = {
            when = { type = "wifi" state = "acquiring" }
            icon = "icon:network-wireless-acquiring-symbolic"
        }
        wifi_connected_none = {
            when = { type = "wifi" state = "connected" signal_strength = 20 }
            icon = "icon:network-wireless-signal-none-symbolic"
        }
        wifi_connected_weak = {
            when = { type = "wifi" state = "connected" signal_strength = 40 }
            icon = "icon:network-wireless-signal-weak-symbolic"
        }
        wifi_connected_ok = {
            when = { type = "wifi" state = "connected" signal_strength = 50 }
            icon = "icon:network-wireless-signal-ok-symbolic"
        }
        wifi_connected_good = {
            when = { type = "wifi" state = "connected" signal_strength = 80 }
            icon = "icon:network-wireless-signal-good-symbolic"
        }
        wifi_connected_excellent = {
            when = { type = "wifi" state = "connected" signal_strength = 100 }
            icon = "icon:network-wireless-signal-excellent-symbolic"
        }
        cellular_disconnected = {
            when = { type = "cellular" state = "disconnected" }
            icon = ""
        }
        cellular_acquiring = {
            when = { type = "cellular" state = "acquiring" }
            icon = "icon:network-cellular-acquiring-symbolic"
        }
        cellular_connected = {
            when = { type = "cellular" state = "connected" }
            icon = "icon:network-cellular-connected-symbolic"
        }
        vpn_disconnected = {
            when = { type = "vpn" state = "disconnected" }
            icon = ""
        }
        vpn_acquiring = {
            when = { type = "vpn" state = "acquiring" }
            icon = "icon:network-vpn-acquiring-symbolic"
        }
        vpn_connected = {
            when = { type = "vpn" state = "connected" }
            icon = "icon:network-vpn-symbolic"
        }
        unknown = {
            when = { type = "unknown" }
            icon = "icon:dialog-question-symbolic"
        }
    }
}
```

</details>


## Styling

| Selector                 | Description                      |
|--------------------------|----------------------------------|
| `.network_manager`       | NetworkManager widget container. |
| `.network_manager .icon` | NetworkManager widget icons.     |

For more information on styling, please see the [styling guide](styling-guide).
