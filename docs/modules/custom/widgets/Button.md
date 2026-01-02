> [!NOTE]
> This page refers to a widget that can be used by the [custom module](/modules/custom/custom).

A clickable button, which can run a command when clicked.
Widgets can be placed inside the button, as with the [Box](box) widget.

## Example

```corn
let {
    $button = { 
        type = "custom" 
        bar = [
            {
                type = "button"
                label = "Click me!"
                command = "!echo 'Hello' > /tmp/output.txt"
            }
        ] 
    }
} in {
    center = [ $button ] 
}
```

## Configuration

> Type `button`

%properties:ButtonWidget%