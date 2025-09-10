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
| Popups          | ✅      | Potential styling issues, otherwise working.                                                          |
| Theming - CSS   | ✅      |                                                                                                       |
| Theming - Icons | ⚠️     | Image sizing/scaling issues. Some icons not resolving.                                                |
| Config - Format | ⚠️ ️   | Angle/justify properties have been removed from widgets and should now be controlled via CSS instead. |
| IPC             | ⚠️     | Some popup-related commands not implemented.                                                          | 

## Modules

| Module          | Status | Notes                                                                                                                                    |
|-----------------|--------|------------------------------------------------------------------------------------------------------------------------------------------|
| Battery         | ✅      |                                                                                                                                          |
| Bindmode        | ❌      |                                                                                                                                          |
| Bluetooth       | ❌      |                                                                                                                                          |
| Cairo           | ✅      |                                                                                                                                          |
| Clipboard       | ✅      |                                                                                                                                          |
| Clock           | ✅      |                                                                                                                                          |
| Custom          | ✅      |                                                                                                                                          |
| Focused         | ✅      |                                                                                                                                          |
| Keyboard        | ✅      |                                                                                                                                          |
| Label           | ✅      |                                                                                                                                          |
| Launcher        | ⚠️     | Window switching behaviour not fully implemented. Popup behaviour may be a little strange.                                               |
| Menu            | ❌      |                                                                                                                                          |
| Music           | ✅      |                                                                                                                                          |
| Network Manager | ✅      |                                                                                                                                          |
| Notifications   | ✅      |                                                                                                                                          |
| Script          | ✅      |                                                                                                                                          |
| SysInfo         | ✅      |                                                                                                                                          |
| Tray            | ❌      | GTK4 removes widgets required to move the tray. No `libdbusmenu-gtk4` either. will need to manually re-create menus with custom widgets. |
| Volume          | ✅      |                                                                                                                                          |
| Workspaces      | ✅      |                                                                                                                                          |
