The below example is a full stylesheet for all modules:

```css
* {
    /* a nerd font is required to be installed for icons */
    font-family: Noto Sans Nerd Font, sans-serif;
    font-size: 16px;
    border: none;
}

#bar {
    border-top: 1px solid #424242;
}

.container {
    background-color: #2d2d2d;
}

.container#end > * + * {
    margin-left: 20px;
}

.popup {
    background-color: #2d2d2d;
    border: 1px solid #424242;
}

#workspaces .item {
    color: white;
    background-color: #2d2d2d;
    border-radius: 0;
}

#workspaces .item.focused {
    box-shadow: inset 0 -3px;
    background-color: #1c1c1c;
}

#workspaces *:not(.focused):hover {
    box-shadow: inset 0 -3px;
}

#launcher .item {
    border-radius: 0;
    background-color: #2d2d2d;
    margin-right: 4px;
}

#launcher .item:not(.focused):hover {
    background-color: #1c1c1c;
}

#launcher .open {
    border-bottom: 2px solid #6699cc;
}

#launcher .focused {
    color: white;
    background-color: black;
    border-bottom: 4px solid #6699cc;
}

#launcher .urgent {
    color: white;
    background-color: #8f0a0a;
}

#script {
    color: white;
}

#sysinfo {
    color: white;
}

#tray .item {
    background-color: #2d2d2d;
}

#mpd {
    background-color: #2d2d2d;
    color: white;
}

#popup-mpd {
    color: white;
    padding: 1em;
}

#popup-mpd #album-art {
    margin-right: 1em;
}

#popup-mpd #title .icon, #popup-mpd #title .label {
    font-size: 1.7em;
}

#popup-mpd #controls * {
    border-radius: 0;
    background-color: #2d2d2d;
    color: white;
}

#popup-mpd #controls *:disabled {
    color: #424242;
}

#clock {
    color: white;
    background-color: #2d2d2d;
    font-weight: bold;
}

#popup-clock {
    padding: 1em;
}

#popup-clock #calendar-clock {
    color: white;
    font-size: 2.5em;
    padding-bottom: 0.1em;
}

#popup-clock #calendar {
    background-color: #2d2d2d;
    color: white;
}

#popup-clock #calendar .header {
    padding-top: 1em;
    border-top: 1px solid #424242;
    font-size: 1.5em;
}

#popup-clock #calendar:selected {
    background-color: #6699cc;
}

#focused {
    color: white;
}
```