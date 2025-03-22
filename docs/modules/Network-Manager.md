Displays the current network connection state of NetworkManager.
Supports wired ethernet, wifi, cellular data and VPN connections among others.

> [!NOTE]
> This module is currently a basic skeleton implementation and only offers the most basic functionality currently. 
> It uses NetworkManager's so-called primary connection, 
> and therefore inherits its limitation of only being able to display the "top-level" connection.
> For example, if we have a VPN connection over a wifi connection it will only display the former, 
> until it is disconnected, at which point it will display the latter.
> A solution to this is currently in the works.

## Configuration

> Type: `network_manager`

| Name        | Type      | Default | Description             |
|-------------|-----------|---------|-------------------------|
| `icon_size` | `integer` | `24`    | Size to render icon at. |

> [!NOTE]
> This module does not support module-level [layout options](module-level-options#layout).

<details>
  <summary>JSON</summary>

  ```json
  {
    "end": [
      {
        "type": "network_manager",
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
  type = "network_manager"
  icon_size = 32
  ```
</details>

<details>
  <summary>YAML</summary>

  ```yaml
  end:
    - type: "network_manager"
      icon_size: 32
  ```
</details>

<details>
  <summary>Corn</summary>

  ```corn
  {
    end = [
      {
        type = "network_manager"
        icon_size = 32
      }
    ]
  }
  ```
</details>

## Styling

| Selector                 | Description                      |
|--------------------------|----------------------------------|
| `.network_manager`       | NetworkManager widget container. |
| `.network_manager .icon` | NetworkManager widget icon.      |

For more information on styling, please see the [styling guide](styling-guide).
