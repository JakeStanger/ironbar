> [!NOTE]
> This page refers to a widget that can be used by the [custom module](/modules/custom/custom).

A progress bar.

## Example

The example below shows progress for the current playing song in MPD,
and displays the elapsed/length timestamps as a label above:

```corn
let {
    $progress = { 
        type = "custom" 
        bar = [
            {
                type = "progress"
                value = "500:mpc | sed -n 2p | awk '{ print $4 }' | grep -Eo '[0-9]+' || echo 0"
                label = "{{500:mpc | sed -n 2p | awk '{ print $3 }'}} elapsed"
                length = 200
            }
        ] 
    }
} in {
    center = [ $progress ]
}
```

## Configuration

> Type: `progress`

%properties:ProgressWidget%