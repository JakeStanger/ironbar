Displays one or more labels containing system information. 

Separating information across several labels allows for styling each one independently. 
Pango markup is supported.

Options can be provided in a token to specify operations, units and formatting.

![Screenshot showing sys-info module with widgets for CPU and memory usage percentages](https://f.jstanger.dev/github/ironbar/modules/sysinfo.png)

## Configuration

> Type: `sys_info`

| Name               | Type                                                       | Default        | Description                                                                                                                    |
|--------------------|------------------------------------------------------------|----------------|--------------------------------------------------------------------------------------------------------------------------------|
| `format`           | `string[]`                                                 | `null`         | Array of strings including formatting tokens. For available tokens see below.                                                  |
| `interval`         | `integer` or `Map`                                         | `5`            | Seconds between refreshing. Can be a single value for all data or a map of individual refresh values for different data types. |
| `interval.memory`  | `integer`                                                  | `5`            | Seconds between refreshing memory data.                                                                                        |
| `interval.cpu`     | `integer`                                                  | `5`            | Seconds between refreshing cpu data.                                                                                           |
| `interval.temps`   | `integer`                                                  | `5`            | Seconds between refreshing temperature data.                                                                                   |
| `interval.disks`   | `integer`                                                  | `5`            | Seconds between refreshing disk data.                                                                                          |
| `interval.network` | `integer`                                                  | `5`            | Seconds between refreshing network data.                                                                                       |
| `orientation`      | `'horizontal'` or `'vertical'` (shorthand: `'h'` or `'v'`) | `'horizontal'` | Orientation of the labels.                                                                                                     |
| `direction`        | `'horizontal'` or `'vertical'` (shorthand: `'h'` or `'v'`) | `'horizontal'` | How the labels are laid out (not the rotation of an individual label).                                                         |

<details>
<summary>JSON</summary>

```json
{
  "end": [
    {
      "format": [
        " {cpu_percent}% | {cpu_frequency} GHz | {temp_c@CPUTIN}°C",
        " {memory_used} / {memory_total} GB ({memory_available} | {memory_percent}%) | {swap_used} / {swap_total} GB ({swap_free} | {swap_percent}%)",
        "󰋊 {disk_used#T@/:.1} / {disk_total#T@/:.1} TB ({disk_percent@/}%) | {disk_read} / {disk_write} MB/s",
        "󰓢 {net_down@enp39s0} / {net_up@enp39s0} Mbps",
        "󰖡 {load_average_1} | {load_average_5} | {load_average_15}",
        "󰥔 {uptime}"
      ],
      "interval": {
        "cpu": 1,
        "disks": 300,
        "memory": 30,
        "networks": 3,
        "temps": 5
      },
      "type": "sys_info"
    }
  ]
}
```

</details>

<details>
<summary>TOML</summary>

```toml
[[end]]
type = 'sys_info'
format = [
    " {cpu_percent}% | {cpu_frequency} GHz | {temp_c@CPUTIN}°C",
    " {memory_used} / {memory_total} GB ({memory_available} | {memory_percent}%) | {swap_used} / {swap_total} GB ({swap_free} | {swap_percent}%)",
    "󰋊 {disk_used#T@/:.1} / {disk_total#T@/:.1} TB ({disk_percent@/}%) | {disk_read} / {disk_write} MB/s",
    "󰓢 {net_down@enp39s0} / {net_up@enp39s0} Mbps",
    "󰖡 {load_average_1} | {load_average_5} | {load_average_15}",
    "󰥔 {uptime}"
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
  - " {cpu_percent}% | {cpu_frequency} GHz | {temp_c@CPUTIN}°C"
  - " {memory_used} / {memory_total} GB ({memory_available} | {memory_percent2}%) | {swap_used} / {swap_total} GB ({swap_free} | {swap_percent}%)"
  - "󰋊 {disk_used#T@/:.1} / {disk_total#T@/:.1} TB ({disk_percent@/}%) | {disk_read} / {disk_write} MB/s"
  - "󰓢 {net_down@enp39s0} / {net_up@enp39s0} Mbps"
  - "󰖡 {load_average_1} | {load_average_5} | {load_average_15}"
  - "󰥔 {uptime}"
  interval:
    cpu: 1
    disks: 300
    memory: 30
    networks: 3
    temps: 5
  type: sys_info
```

</details>

<details>
<summary>Corn</summary>

```corn
{
  end = [
    {
      type = "sys_info"

      interval.memory = 30
      interval.cpu = 1
      interval.temps = 5
      interval.disks = 300
      interval.networks = 3

      format = [
        " {cpu_percent}% | {cpu_frequency} GHz | {temp_c@CPUTIN}°C"
        " {memory_used} / {memory_total} GB ({memory_available} | {memory_percent2}%) | {swap_used} / {swap_total} GB ({swap_free} | {swap_percent}%)"
        "󰋊 {disk_used#T@/:.1} / {disk_total#T@/:.1} TB ({disk_percent@/}%) | {disk_read} / {disk_write} MB/s"
        "󰓢 {net_down@enp39s0} / {net_up@enp39s0} Mbps"
        "󰖡 {load_average_1} | {load_average_5} | {load_average_15}"
        "󰥔 {uptime}"
      ]
    }
  ]
}
```

</details>

### Formatting Tokens

The below table lists the tokens which can be used in the `format` configuration option. 
More information about each of these and the additional options can be found further below.

| Token                    | Default Function | Default Unit | Default Formatting |
|--------------------------|------------------|--------------|--------------------|
| **CPU**                  |                  |              |                    | 
| `{cpu_frequency[@core]}` | `mean`           | MHz          | `.2`               | 
| `{cpu_percent[@core]}`   | `mean`           | %            | `0<2`              | 
| **Memory**               |                  |              |                    | 
| `{memory_free}`          | N/A              | GB           | `0<4.1`            | 
| `{memory_available}`     | N/A              | GB           | `0<4.1`            | 
| `{memory_used}`          | N/A              | GB           | `0<4.1`            | 
| `{memory_total}`         | N/A              | GB           | `0<4.1`            | 
| `{memory_percent}`       | N/A              | GB           | `0<4.1`            |
| `{swap_free}`            | N/A              | GB           | `0<4.1`            | 
| `{swap_used}`            | N/A              | GB           | `0<4.1`            | 
| `{swap_total}`           | N/A              | GB           | `0<4.1`            | 
| `{swap_percent}`         | N/A              | GB           | `0<4.1`            | 
| **Temperature**          |                  |              |                    | 
| `{temp_c[@sensor]}`      | `max`            | °C           |                    | 
| `{temp_f[@sensor]}`      | `max`            | °F           |                    | 
| **Disk**                 |                  |              |                    | 
| `{disk_free[@mount]}`    | `sum`            | GB           |                    | 
| `{disk_used[@mount]}`    | `sum`            | GB           |                    | 
| `{disk_total[@mount]}`   | `sum`            | GB           |                    | 
| `{disk_percent[@mount]}` | `sum`            | %            |                    | 
| `{disk_read[@mount]}`    | `sum`            | MB/s         |                    | 
| `{disk_write[@mount]}`   | `sum`            | MB/s         |                    | 
| **Network**              |                  |              |                    | 
| `{net_down[@adapter]}`   | `sum`            | Mb/s         |                    | 
| `{net_up[@adapter]}`     | `sum`            | Mb/s         |                    | 
| **System**               |                  |              |                    | 
| `{load_average_1}`       | N/A              | -            | `.2`               | 
| `{load_average_5}`       | N/A              | -            | `.2`               | 
| `{load_average_15}`      | N/A              | -            | `.2`               | 
| `{uptime}`               | N/A              | ???          | ???                |

#### Functions and names

Many of the tokens operate on a value set, as opposed to an individual value:

- CPU tokens operate on each physical thread.
- Temperature tokens operate on each sensor.
- Disk tokens operate on each mount.
- Network tokens operate on each adapter.

By default, these will apply a function to the full set to reduce them down to a single value. 
The list of available functions is shown below:

| Function | Description                             |
|----------|-----------------------------------------|
| `sum`    | Adds each value in the set.             |
| `min`    | Gets the smallest value in the set.     |
| `max`    | Gets the largest value in the set.      |
| `mean`   | Gets the mean average value of the set. |

It is also possible to get only a single value from the set by specifying a name instead of a function.

| Token category | Valid name                               |
|----------------|------------------------------------------|
| CPU            | A CPU thread, eg `cpu0`, `cpu1`, ...     |
| Temperature    | A sensor name, eg `CPUTIN`.              |
| Disk           | A disk mountpoint, eg `/`, `/home`, ...  |
| Network        | An adapter name, eg `eth0` or `enp30s0`. |


To specify a name or function, use a `@`. For example, to show disk percent for `/home`:

```json
"{disk_percent@/home}%"
```

To show total CPU utilization where each core represents 100% (like `htop` etc):

```json
"{cpu_percent@sum}%"
```

> [!TIP]
> Available values can be queried over IPC using the CLI.
> This can be particularly useful for sensors, which tend not to have obvious names.
> 
> ```shell
> ironbar var list sysinfo.temp_c
> ```
> 
> Some usual cases to look out for:
> 
> - `k10temp` is an AMD CPU internal sensor
> - Motherboard chipsets tend to prefix their sensors accordingly. For example, `CPUTIN`, `nct6687 CPU`, `asusec AMD`.
> - `amdgpu` is as it suggests.
> 
> Sensor names are pulled from `hwmon` and should vaguely line up with the output of `sensors`

#### Prefixes and units

For tokens which return an appropriate unit, you can specify the SI prefix (or unit in some special cases).
The following options can be supplied:

| Name    | Value |
|---------|-------|
| Kilo    | `k`   |
| Mega    | `M`   |
| Giga    | `G`   |
| Tera    | `T`   |
| Peta    | `P`   |
|         |       |
| Kibi    | `ki`  |
| Mebi    | `Mi`  |
| Gibi    | `Gi`  |
| Tebi    | `Ti`  |
| Pebi    | `Pi`  |
|         |       | 
| Kilobit | `kb`  |
| Megabit | `Mb`  |
| Gigabit | `Gb`  |

To specify a prefix or unit, use a `#`. For example, to show free total disk space in terabytes:

```json
"{disk_free#T} TB"
```

#### Formatting

To control the formatting of the resultant number, 
a subset of Rust's string formatting is implemented. This includes:

- Width
- Fill/Alignment
- Precision

Formatting is specified with a `:` and MUST be the last part of a token.

##### Width

The width controls the minimum string length of the value. 
Specifying just a width will left-pad the value with `0` until the value reaches the target length.

The width can be any value from `1-9`. Larger values are not supported.

For example, to render CPU usage as `045%`:

```json
"{cpu_usage:3}%"
```

##### Fill/Alignment

These options can be used to control the `width` property.

To specify the fill and alignment, prefix the width with a character and a direction.
Fill characters can be any single UTF-8 character EXCEPT 1-9. Alignment must be one of:

- `<` - Left fill
- `^` - Center fill
- `>` - Right fill

For example, to render CPU usage as ` 45%`:

```json
"{cpu_usage: <3}%"
```

##### Precision

The number of decimal places a value is shown to can be controlled using precision.
Any value is supported.

To specify precision, include a `.` followed by the value. If other options are supplied, this MUST come after.

For example, to render used disk space to 2dp:

```json
"{disk_used:.2} GB"
```

---

#### Combining Options

Each of the token options can be combined to create more complex solutions.

Putting it all together, you could show the free disk space on your `/home` partition in terabytes,
left-padded with spaces to a min width of 5, and shown to 2dp as follows:

```json
"{disk_used@/home#T: <5.2} TB"
```

## Styling

| Selector         | Description                  |
|------------------|------------------------------|
| `.sysinfo`       | Sysinfo widget box           |
| `.sysinfo .item` | Individual information label |

For more information on styling, please see the [styling guide](styling-guide).
