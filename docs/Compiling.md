You can compile Ironbar from source using `cargo`. 
Just clone the repo and build:

```sh
git clone https://github.com/jakestanger/ironbar.git
cd ironbar
cargo build --release
# change path to wherever you want to install
install target/release/ironbar ~/.local/bin/ironbar
```

## Features

By default, all features are enabled for convenience. This can result in a significant compile time.
If you know you are not going to need all the features, you can compile with only the features you need.

As of `v0.10.0`, compiling with no features is about 33% faster. 
On a 3800X, it takes about 60 seconds for no features and 90 seconds for all. 
This difference is expected to increase as the bar develops. 

Features containing a `+` can be stacked, for example `config+json` and `config+yaml` could both be enabled.

To build using only specific features, disable default features and pass a comma separated list to `cargo build`:

```shell
cargo build --release --no-default-features \
  --features http,config+json,clock
```

> ⚠ Make sure you enable at least one `config` feature otherwise you will not be able to start the bar!

| Feature             | Description                                                                       |
|---------------------|-----------------------------------------------------------------------------------|
| **Core**            |                                                                                   |
| http                | Enables HTTP features. Currently this includes the ability to load remote images. |
| config+all          | Enables support for all configuration languages.                                  |
| config+json         | Enables configuration support for JSON.                                           |
| config+yaml         | Enables configuration support for YAML.                                           |
| config+toml         | Enables configuration support for TOML.                                           |
| config+corn         | Enables configuration support for [Corn](https://github.com/jakestanger.corn).    |
| **Modules**         |                                                                                   |
| clock               | Enables the `clock` module.                                                       |
| music+all           | Enables the `music` module with support for all player types.                     |
| music+mpris         | Enables the `music` module with MPRIS support.                                    |
| music+mpd           | Enables the `music` module with MPD support.                                      |
| sys_info            | Enables the `sys_info` module.                                                    |
| tray                | Enables the `tray` module.                                                        |
| workspaces+all      | Enables the `workspaces` module with support for all compositors.                 |
| workspaces+sway     | Enables the `workspaces` module with support for Sway.                            |
| workspaces+hyprland | Enables the `workspaces` module with support for Hyprland.                        |

