> [!NOTE]
> This page refers to a widget that can be used by the [custom module](/modules/custom/custom).

An image or icon from disk or http.

## Example

```corn
let {
    $image = { 
        type = "custom" 
        bar = [
            {
                type = "image"
                src = "file:///home/user/icon.png"
            }
        ] 
    }
} in {
    center = [ $image ] 
}
```

## Configuration

> Type `image`

%properties:ImageWidget%