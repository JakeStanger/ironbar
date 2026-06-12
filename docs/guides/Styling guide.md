Ironbar ships with no styles by default, so will fall back to the default GTK styles.

To style the bar, create a file at `~/.config/ironbar/style.css`. This default path can be overridden by using the `IRONBAR_CSS` environment variable.

Style changes are hot-loaded so there is no need to reload the bar.

Since the bar is GTK-based, it uses [GTK's implementation of CSS](https://docs.gtk.org/gtk4/css-overview.html),
which only includes a subset of the full web spec (plus a few non-standard properties).

> [!TIP]
> The use of GTK4 does not imply the use of `libadwaita`.
> Many GTK4 apps do use the library, so this can be confusing.
> Any `libadwaita` based themes or configuration will not apply to Ironbar.
> GTK4 themes will apply to Ironbar.

The below table describes the selectors provided by the bar itself.
Information on styling individual modules can be found on their pages in the sidebar.

| Selector            | Description                                |
|---------------------|--------------------------------------------|
| `.background`       | Top-level window.                          |
| `#bar`              | Bar root box.                              |
| `#bar #start`       | Bar left or top modules container box.     |
| `#bar #center`      | Bar center modules container box.          |
| `#bar #end`         | Bar right or bottom modules container box. |
| `.container`        | All of the above.                          |
| `.widget-container` | The `EventBox` wrapping any widget.        |
| `.widget`           | Any widget.                                |
| `.popup`            | Any popup box.                             |

Every Ironbar widget can be selected using a `kebab-case` class name matching its name. 
You can also target popups by prefixing `popup-` to the name. For example, you can use `.clock` and `.popup-clock` respectively.

Setting the `name` option on a widget allows you to target that specific instance using `#name`. 
You can also add additional classes to re-use styles. In both cases, `popup-` is automatically prefixed to the popup (`#popup-name` or `.popup-my-class`).

You can also target all GTK widgets of a certain type directly using their name. For example, `label` will select all labels, and `button:hover` will select the hover state on *all* buttons.
These names are all lower case with no separator, so `MenuBar` -> `menubar`.

> [!NOTE]
> If an entry takes no effect you might have to use a more specific selector. 
> For example, attempting to set text size on `.popup-clipboard .item` will likely have no effect. 
> Instead, you can target the more specific `.popup-clipboard .item label`. 

Running `ironbar inspect` can be used to find out how to address an element.

GTK CSS does not support custom properties, but it does have its own custom `@define-color` syntax which you can use for re-using colours:

```css
@define-color color_bg #2d2d2d;

box, menubar {
    background-color: @color_bg;
}
```
