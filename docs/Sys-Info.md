Displays one or more labels containing system information. 

Separating information across several labels allows for styling each one independently. 
Pango markup is supported.

![Screenshot showing sys-info module with widgets for all of the types of formatting tokens](https://user-images.githubusercontent.com/5057870/196059090-4056d083-69f0-4e6f-9673-9e35dc29d9f0.png)


## Configuration

> Type: `sys-info`

| Name               | Type               | Default | Description                                                                                                                    |
|--------------------|--------------------|---------|--------------------------------------------------------------------------------------------------------------------------------|
| `format`           | `string[]`         | `null`  | Array of strings including formatting tokens. For available tokens see below.                                                  |
| `interval`         | `integer` or `Map` | `5`     | Seconds between refreshing. Can be a single value for all data or a map of individual refresh values for different data types. |
| `interval.memory`  | `integer`          | `5`     | Seconds between refreshing memory data                                                                                         |
| `interval.cpu`     | `integer`          | `5`     | Seconds between refreshing cpu data                                                                                            |
| `interval.temps`   | `integer`          | `5`     | Seconds between refreshing temperature data                                                                                    |
| `interval.disks`   | `integer`          | `5`     | Seconds between refreshing disk data                                                                                           |
| `interval.network` | `integer`          | `5`     | Seconds between refreshing network data                                                                                        |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "format": [
        " {cpu-percent}% | {temp-c:k10temp-Tccd1}°C",
        " {memory-used} / {memory-total} GB ({memory-percent}%)",
        "| {swap-used} / {swap-total} GB ({swap-percent}%)",
        " {disk-used:/} / {disk-total:/} GB ({disk-percent:/}%)",
        "李 {net-down:enp39s0} / {net-up:enp39s0} Mbps",
        "猪 {load-average:1} | {load-average:5} | {load-average:15}",
        " {uptime}"
      ],
      "interval": {
        "cpu": 1,
        "disks": 300,
        "memory": 30,
        "networks": 3,
        "temps": 5
      },
      "type": "sys-info"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = 'sys-info'
format = [
    ' {cpu-percent}% | {temp-c:k10temp-Tccd1}°C',
    ' {memory-used} / {memory-total} GB ({memory-percent}%)',
    '| {swap-used} / {swap-total} GB ({swap-percent}%)',
    ' {disk-used:/} / {disk-total:/} GB ({disk-percent:/}%)',
    '李 {net-down:enp39s0} / {net-up:enp39s0} Mbps',
    '猪 {load-average:1} | {load-average:5} | {load-average:15}',
    ' {uptime}',
]

[end.interval]
cpu = 1
disks = 300
memory = 30
networks = 3
temps = 5


```

</details>

<details>
<summary>YAML</summary>

```yaml
end:
- format:
  -  {cpu-percent}% | {temp-c:k10temp-Tccd1}°C
  -  {memory-used} / {memory-total} GB ({memory-percent}%)
  - '| {swap-used} / {swap-total} GB ({swap-percent}%)'
  -  {disk-used:/} / {disk-total:/} GB ({disk-percent:/}%)
  - 李 {net-down:enp39s0} / {net-up:enp39s0} Mbps
  - 猪 {load-average:1} | {load-average:5} | {load-average:15}
  -  {uptime}
  interval:
    cpu: 1
    disks: 300
    memory: 30
    networks: 3
    temps: 5
  type: sys-info
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "sys-info"

      interval.memory = 30
      interval.cpu = 1
      interval.temps = 5
      interval.disks = 300
      interval.networks = 3

      format = [
        " {cpu-percent}% | {temp-c:k10temp-Tccd1}°C"
        " {memory-used} / {memory-total} GB ({memory-percent}%)"
        "| {swap-used} / {swap-total} GB ({swap-percent}%)"
        " {disk-used:/} / {disk-total:/} GB ({disk-percent:/}%)"
        "李 {net-down:enp39s0} / {net-up:enp39s0} Mbps"
        "猪 {load-average:1} | {load-average:5} | {load-average:15}"
        " {uptime}"
      ]
    }
  ]
}
```

</details>

### Formatting Tokens

The following tokens can be used in the `format` configuration option:

| Token                    | Description                                                                        |
|--------------------------|------------------------------------------------------------------------------------|
| **CPU**                  |                                                                                    |
| `{cpu-percent}`          | Total CPU utilisation percentage                                                   |
| **Memory**               |                                                                                    |
| `{memory-free}`          | Memory free in GB.                                                                 |
| `{memory-used}`          | Memory used in GB.                                                                 |
| `{memory-total}`         | Memory total in GB.                                                                |
| `{memory-percent}`       | Memory utilisation percentage.                                                     |
| `{swap-free}`            | Swap free in GB.                                                                   |
| `{swap-used}`            | Swap used in GB.                                                                   |
| `{swap-total}`           | Swap total in GB.                                                                  |
| `{swap-percent}`         | Swap utilisation percentage.                                                       |
| **Temperature**          |                                                                                    |
| `{temp-c:[sensor]}`      | Temperature in degrees C. Replace `[sensor]` with the sensor label.                |
| `{temp-f:[sensor]}`      | Temperature in degrees F. Replace `[sensor]` with the sensor label.                |
| **Disk**                 |                                                                                    |
| `{disk-free:[mount]}`    | Disk free space in GB. Replace `[mount]` with the disk mountpoint.                 |
| `{disk-used:[mount]}`    | Disk used space in GB. Replace `[mount]` with the disk mountpoint.                 |
| `{disk-total:[mount]}`   | Disk total space in GB. Replace `[mount]` with the disk mountpoint.                |
| `{disk-percent:[mount]}` | Disk utilisation percentage. Replace `[mount]` with the disk mountpoint.           |
| **Network**              |                                                                                    |
| `{net-down:[adapter]}`   | Average network download speed in Mbps. Replace `[adapter]` with the adapter name. |
| `{net-up:[adapter]}`     | Average network upload speed in Mbps. Replace `[adapter]` with the adapter name.   |
| **System**               |                                                                                    |
| `{load-average:1}`       | 1-minute load average.                                                             |
| `{load-average:5}`       | 5-minute load average.                                                             |
| `{load-average:15}`      | 15-minute load average.                                                            |
| `{uptime}`               | System uptime formatted as `HH:mm`.                                                |

## Styling

| Selector         | Description                  |
|------------------|------------------------------|
| `#sysinfo`       | Sysinfo widget box           |
| `#sysinfo #item` | Individual information label |