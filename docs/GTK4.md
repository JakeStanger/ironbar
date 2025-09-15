As the GTK3 and gtk-layer-shell crates are now deprecated, there is a need to move to GTK 4.

The `refactor/gtk-4` branch and PR [#112](https://github.com/JakeStanger/ironbar/pull/112) are tracking the code upgrade.
This page documents the port progress.

Assistance in the porting process is very much welcomed, no matter how small.

All feature flags have been ported and will build/run properly. Known bugs are documented below.

## Core functionality

| Area                     | Status | Notes                                                                                                                                |
|--------------------------|--------|--------------------------------------------------------------------------------------------------------------------------------------|
| Bar                      | ✅      |                                                                                                                                      |
| Popups                   | ✅      | Potential styling issues, otherwise working.                                                                                         |
| Theming - CSS            | ✅      |                                                                                                                                      |
| Theming - Images & Icons | ✅      |                                                                                                                                      |
| Config - Format          | ⚠️ ️   | Angle properties have been removed from widgets and should now be controlled via CSS instead. Some options may not work as expected. |
| IPC                      | ⚠️     | Some popup-related commands not implemented.                                                                                         | 

## Modules

| Module          | Status | Notes                                                                                           |
|-----------------|--------|-------------------------------------------------------------------------------------------------|
| Battery         | ✅      |                                                                                                 |
| Bindmode        | ✅      |                                                                                                 |
| Bluetooth       | ✅      |                                                                                                 |
| Cairo           | ✅      |                                                                                                 |
| Clipboard       | ✅      |                                                                                                 |
| Clock           | ✅      |                                                                                                 |
| Custom          | ✅      |                                                                                                 |
| Focused         | ✅      |                                                                                                 |
| Keyboard        | ✅      |                                                                                                 |
| Label           | ✅      |                                                                                                 |
| Launcher        | ️✅     |                                                                                                 |
| Menu            | ⚠️     | Buggy. Issues with submenus all showing at once, and setting size options not working properly. |
| Music           | ✅      |                                                                                                 |
| Network Manager | ✅      |                                                                                                 |
| Notifications   | ✅      |                                                                                                 |
| Script          | ✅      |                                                                                                 |
| SysInfo         | ✅      |                                                                                                 |
| Tray            | ⚠️     | Some known issues activating menu items in some applications.                                   |
| Volume          | ✅      |                                                                                                 |
| Workspaces      | ✅      |                                                                                                 |
