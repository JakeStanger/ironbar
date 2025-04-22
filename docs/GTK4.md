As the GTK3 and gtk-layer-shell crates are now deprecated, there is a need to move to GTK 4.

The `refactor/gtk-4` branch and PR [#112](https://github.com/JakeStanger/ironbar/pull/112) are tracking the code upgrade.
This page documents the port progress.

Assistance in the porting process is very much welcomed, no matter how small.

As many modules have not been ported, the default feature set will fail to compile. 
It is therefore necessary to compile manually with `--no-default-features`, enabling only the working modules:

```shell
cargo run --no-default-features \
  --features config+all,clock,cairo
```

A full list of feature flags can be found [here](Compiling#features).

## Core functionality

| Area            | Status | Notes                                                                                                 |
|-----------------|--------|-------------------------------------------------------------------------------------------------------|
| Bar             | ✅      |                                                                                                       |
| Popups          | ⚠️     | Display in correct position but suffer from minor focus-steal issue on close, and other edge cases.   |
| Theming - CSS   | ✅      |                                                                                                       |
| Theming - Icons | ⚠️     | GTK4 does not support icon theming - always uses default theme. Image scaling may be incorrect.       |
| Config - Format | ❌ ️    | Angle/justify properties have been removed from widgets and should now be controlled via CSS instead. |

## Modules

| Module          | Status | Notes                                                                                                                                    |
|-----------------|--------|------------------------------------------------------------------------------------------------------------------------------------------|
| Cairo           | ✅      |                                                                                                                                          |
| Clipboard       | ✅      |                                                                                                                                          |
| Clock           | ✅      |                                                                                                                                          |
| Custom          | ✅      |                                                                                                                                          |
| Focused         | ✅      |                                                                                                                                          |
| Keyboard        | ✅      |                                                                                                                                          |
| Label           | ✅      |                                                                                                                                          |
| Launcher        | ❌      |                                                                                                                                          |
| Music           | ✅      |                                                                                                                                          |
| Network Manager | ❌      |                                                                                                                                          |
| Notifications   | ✅      |                                                                                                                                          |
| Script          | ✅      |                                                                                                                                          |
| Sway Mode       | ❌      |                                                                                                                                          |
| SysInfo         | ✅      |                                                                                                                                          |
| Tray            | ❌      | GTK4 removes widgets required to move the tray. No `libdbusmenu-gtk4` either. will need to manually re-create menus with custom widgets. |
| UPower          | ❌      |                                                                                                                                          |
| Volume          | ❌      |                                                                                                                                          |
| Workspaces      | ✅      |                                                                                                                                          |
