> [!NOTE]
> This page refers to a widget that can be used by the [custom module](/modules/custom/custom).

A draggable slider (also sometimes known as a scale).

## Example

The example slider widget below shows a volume control for MPC,
which updates the server when changed, and polls the server for volume changes to keep the slider in sync.

```corn
let {
    $slider = { 
        type = "custom" 
        bar = [
            {
                type = "slider"
                length = 100
                max = 100
                on_change="!mpc volume ${0%.*}"
                value = "200:mpc volume | cut -d ':' -f2 | cut -d '%' -f1"
            }
        ] 
    }
} in {
    center = [ $slider ] 
}
```

## Configuration

> Type: `slider`

%{properties:SliderWidget}%