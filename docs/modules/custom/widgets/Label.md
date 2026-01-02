> [!NOTE]
> This page refers to a widget that can be used by the [custom module](/modules/custom/custom).

A text label. This can be used for displaying dynamic content.

## Example

```corn
let {
    $label = { 
        type = "custom" 
        bar = [
            {
                type = "label"
                label = "Hello, world!"
            }
        ] 
    }
} in {
    center = [ $label ] 
}
```

## Configuration

> Type `label`

%properties:LabelWidget%