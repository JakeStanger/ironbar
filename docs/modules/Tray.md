Displays a fully interactive icon tray using the KDE `libappindicator` protocol. 

![Screenshot showing icon tray widget](https://user-images.githubusercontent.com/5057870/184540135-78ffd79d-f802-4c79-b09a-05a733dadc55.png)

## Configuration

> Type: `tray`

***This module provides no configuration options.***

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "type": "tray"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = "tray"
```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
  - type: "tray"
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    { type = "tray" }
  ]
}
```

</details>

## Styling

| Selector      | Description      |
|---------------|------------------|
| `#tray`       | Tray widget box  |
| `#tray .item` | Tray icon button |
