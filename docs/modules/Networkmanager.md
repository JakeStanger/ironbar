Displays the current network connection state of NetworkManager. Supports wired ethernet, wifi,
cellular data and VPN connections among others.

This module uses NetworkManager's so-called primary connection, and therefore inherits its
limitation of only being able to display the "top-level" connection. For example, if we have a VPN
connection over a wifi connection it will only display the former, until it is disconnected, at
which point it will display the latter. A solution to this is currently in the works.

## Configuration

> Type: `networkmanager`

| Name        | Type      | Default | Description             |
|-------------|-----------|---------|-------------------------|
| `icon_size` | `integer` | `24`    | Size to render icon at. |

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
|------------------------|----------------------------------|
| `.networkmanager`      | NetworkManager widget container. |
| `.networkmanger .icon` | NetworkManager widget icon.      |

For more information on styling, please see the [styling guide](styling-guide).
