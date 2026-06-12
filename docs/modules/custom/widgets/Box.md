> [!NOTE]
> This page refers to a widget that can be used by the [custom module](/modules/custom/custom).

A container to place nested widgets inside. 

## Example

```corn
let {
    $box = { 
        type = "custom" 
        bar = [
            {
                type = "box"
                widgets = []
            }
        ] 
    }
} in {
    center = [ $box ] 
}
```

## Configuration

> Type: `box`

%{properties:BoxWidget}%