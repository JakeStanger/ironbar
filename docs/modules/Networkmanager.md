Displays network connectivity information. Requires NetworkManager.

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
