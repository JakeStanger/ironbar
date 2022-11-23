# Ironbar

Ironbar is a customisable and feature-rich bar for wlroots compositors, written in Rust.
It uses GTK3 and gtk-layer-shell.

The bar can be styled to your liking using CSS and hot-loads style changes.
For information and examples on styling please see the [wiki](https://github.com/JakeStanger/ironbar/wiki).

![Screenshot of fully configured bar with MPD widget open](https://user-images.githubusercontent.com/5057870/184539623-92d56a44-a659-49a9-91f9-5cdc453e5dfb.png)


## Installation

### Cargo

```sh
cargo install ironbar
```

[crate](https://crates.io/crates/ironbar)

### Arch Linux

```sh
yay -S ironbar-git
```

[aur package](https://aur.archlinux.org/packages/ironbar-git)

### Nix Flake
```nix
# Add the ironbar flake input
inputs.ironbar.url = "github:JakeStanger/ironbar";

# And add the home-manager module
inputs.ironbar.homeManagerModules.default

# And configure
programs.ironbar = {
    enable = true;
    config = {};
    style = "";
};
```

### Source

```sh
git clone https://github.com/jakestanger/ironbar.git
cd ironbar
cargo build --release
# change path to wherever you want to install
install target/release/ironbar ~/.local/bin/ironbar
```

[repo](https://github.com/jakestanger/ironbar)

## Running

All of the above installation methods provide a binary called `ironbar`.

You can set the `IRONBAR_LOG` or `IRONBAR_FILE_LOG` environment variables to 
`error`, `warn`, `info`, `debug` or `trace` to configure the log output level.
These default to `IRONBAR_LOG=info` and `IRONBAR_FILE_LOG=error`.
File output can be found at `~/.local/share/ironbar/error.log`.

## Configuration

Ironbar gives a lot of flexibility when configuring, including multiple file formats
and options for scaling complexity: you can use a single config across all monitors,
or configure different/multiple bars per monitor.

A full configuration guide can be found [here](https://github.com/JakeStanger/ironbar/wiki/configuration-guide).

## Styling

To get started, create a stylesheet at `.config/ironbar/style.css`. Changes will be hot-reloaded every time you save the
file.

A full styling guide can be found [here](https://github.com/JakeStanger/ironbar/wiki/styling-guide).

## Project Status

This project is in alpha, but should be usable.
Everything that is implemented works and should be documented.
Proper error handling is in place so things should either fail gracefully with detail, or not fail at all.

There is currently room for lots more modules, and lots more configuration options for the existing modules.
The current configuration schema is not set in stone and breaking changes could come along at any point;
until the project matures I am more interested in ease of use than backwards compatibility.

A few bugs do exist, and I am sure there are plenty more to be found.

The project will be *actively developed* as I am using it on my daily driver.
Bugs will be fixed, features will be added, code will be refactored.

## Contribution Guidelines

Please check [here](https://github.com/JakeStanger/ironbar/blob/master/CONTRIBUTING.md).

## Acknowledgements

- [Waybar](https://github.com/Alexays/Waybar) - A lot of the initial inspiration, and a pretty great bar.
- [Rustbar](https://github.com/zeroeightysix/rustbar) - Served as a good demo for writing a basic GTK bar in Rust
- [Smithay Client Toolkit](https://github.com/Smithay/client-toolkit) - Essential in being able to communicate to Wayland
