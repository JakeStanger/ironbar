Ironbar ships with no styles by default, so will fall back to the default GTK styles.

To style the bar, create a file at `~/.config/ironbar/style.css`.

Style changes are hot-loaded so there is no need to reload the bar.

A reminder: since the bar is GTK-based, it uses GTK's implementation of CSS,
which only includes a subset of the full web spec (plus a few non-standard properties).

The below table describes the selectors provided by the bar itself.
Information on styling individual modules can be found on their pages in the sidebar.

| Selector       | Description                               |
|----------------|-------------------------------------------|
| `.background`  | Top-level window                          |
| `#bar`         | Bar root box                              |
| `#bar #start`  | Bar left or top modules container box     |
| `#bar #center` | Bar center modules container box          |
| `#bar #end`    | Bar right or bottom modules container box |
| `.container`   | All of the above                          |
