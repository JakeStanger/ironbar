Ironbar now runs on GTK 4. While the migration should be fairly seamless, this does bring some important differences.

This guide aims to be as comprehensive as it reasonably can be, and will be updated if more notable differences arise.

For anybody involved in developing Ironbar, a more technical guide is available from GTK:
<https://docs.gtk.org/gtk4/migrating-3to4.html>

## Building

The build dependencies have changed:

- Ironbar now requires `gtk4` and `gtk4-layer-shell` in place of their gtk3 counterparts.
- `libdbusmenu-gtk3` is no longer required. There is no GTK4 equivalent.

See [Compiling from source](compiling) for more info.

## Popups

Ironbar's GTK3 implementation for popups was, frankly, a hack. 
They were separate layer-shell windows with a margin to offset them into place, and not proper Wayland XDG popups.
This worked well enough, but was never going to last.

GTk 4 removed one of the events required to make this work nicely, so the old approach is dead. 
In its place, proper popups! This comes with a few changes:

- Popup styling has changed a bit. The old `.popup` class and `#popup-<name>` are still in place and still target the contents box,  
  but you'll want to target `popover` and `popover contents` widgets to style the outer window.
- Popups no longer close when the mouse leaves. There are two modes to control how it is closed:
  - If `popup_autohide` is enabled, clicking outside the popup will close it. 
    On some compositors (Hyprland...) this has a nasty habit of aggressively stealing focus.
  - If `popup_autohide` is disabled, clicking the module on a bar is required to close the popup.

This can be a bit finicky with the `launcher` module, but work is happening to improve that in the near future.

## Angle

GTK 4 does away with the `angle` property on widgets. This has two effects:

1. Widgets are no longer automatically rotated on vertical bars.
2. Setting the `orientation` property on a widget will no longer rotate it.

Instead, rotation is now handled in CSS.

```css
.label {
  transform: rotate(90deg);
}
```

## Images

Most cases of the `Image` widget have been replaced with the `Picture` widget.
This makes use of loading textures directly into the GPU.
Any direct references to the widget in CSS will need to be updated.

## Clipboard

The clipboard popup now uses `CheckButton` instead of `RadioButton` widgets.
Any direct references to the widget in CSS will need to be updated.

## Menu

There are some slight changes to how the menu widget sizes itself.
You may need to adjust your `width` and `height` values.
Ideally, both should be specified.

## Tray

The tray module no longer depends on `libdbusmenu-gtk3` for obvious reasons. 
Instead, it is now a native implementation with its UI fully integrated into Ironbar*.

Functionality should be close to on-par with the GTK3 version, apart from a few edge cases.
Notably, GTK 4 menus do not allow you to show icons and text on the same item for some strange reason. 
This means menu items that should have icons next to them will not.

Additionally, since the UI structure for the menu has changed entirely, 
CSS targeting the old menu is expected to break. 
You should be able to style the menu using the `popover` and `contents` widgets.

\* Technically it is using a GTK popover menu and GDK menu model, so not *fully*. 
It does exist entirely in-process and is visible in the inspector at least.

## Volume

The volume popup device picker now uses a `Dropdown` instead of a `Combobox`.
Any direct references to the widget in CSS will need to be updated.



## New features

### CSS

GTK 4 comes with a larger CSS spec that should unlock some new options for styling your bar. This includes:

- Custom properties
- An expanded colour syntax
- More control over icons
- Transforms
- Media queries for light/dark themes (GTK >= 4.20)

The full spec can be found here: <https://docs.gtk.org/gtk4/css-properties.html>

### Configuration

It is now possible to use full display names in place of adapter names when configuring the top-level `monitors` object.

This means you can for example setup:

```corn
{
    monitors.DP-1.start = {}
    monitors.'ASUSTek COMPUTER INC PA278QV M4LMQS060475' = {}
}
```

When using the display name, this looks for a partial match using `starts_with`. 
This allows you to only supply a prefix, and omit part of the end of the name as convenient (for example, if it ends with the adapter name).

### Popup

A new bar-level `popup_autohide` option has been added to control popup close behaviour.

### Clock

The calendar in the clock module's popup has some more styling options, detailed [here](https://docs.gtk.org/gtk4/class.Calendar.html#css-nodes).
Most importantly, there is now a `.today` class.