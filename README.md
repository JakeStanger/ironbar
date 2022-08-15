# Ironbar

Ironbar is a customisable and feature-rich bar targeting the Sway compositor, written in Rust. 
It uses GTK3 and gtk-layer-shell.

The bar can be styled to your liking using CSS and hot-loads style changes. 
For information and examples on styling please see the [wiki](https://github.com/JakeStanger/ironbar/wiki).

![Screenshot of fully configured bar with MPD widget open](https://user-images.githubusercontent.com/5057870/184539623-92d56a44-a659-49a9-91f9-5cdc453e5dfb.png)

## Installation

Run using `ironbar`.

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

## Configuration

Ironbar gives a lot of flexibility when configuring, including multiple file formats 
and options for scaling complexity: you can use a single config across all monitors, 
or configure different/multiple bars per monitor. 

A full configuration guide can be found [here](https://github.com/JakeStanger/ironbar/wiki/configuration-guide).

## Styling

To get started, create a stylesheet at `.config/ironbar/style.css`. Changes will be hot-reloaded every time you save the file.

A full styling guide can be found [here](https://github.com/JakeStanger/ironbar/wiki/styling-guide).

## Project Status

This project is in very early stages:

- Error handling is barely implemented - expect crashes
- There will be bugs!
- Lots of modules need more configuration options
- There's room for lots of modules
- The code is messy and quite prototypal in places
- Config options aren't set in stone - expect breaking changes
- Documentation is probably missing in lots of places

That said, it will be *actively developed* as I am using it on my daily driver.
Bugs will be fixed, features will be added, code will be refactored.

## Contribution Guidelines

I welcome contributions of any kind with open arms. That said, please do stick to some basics:

- For code contributions:
  - Fix any `cargo clippy` warnings, using at least the default configuration.
  - Make sure your code is formatted using `cargo fmt`.
  - Keep any documentation up to date.
  - I won't enforce it, but preferably stick to [conventional commit](https://www.conventionalcommits.org/en/v1.0.0/) messages.


- For PRs:
  - Please open an issue or discussion beforehand. 
    I'll accept most contributions, but it's best to make sure you're not working on something that won't get accepted :)


- For issues:
  - Please provide as much information as you can - share your config, any logs, steps to reproduce...

## Acknowledgements

- [Waybar](https://github.com/Alexays/Waybar) - A lot of the initial inspiration, and a pretty great bar.
- [Rustbar](https://github.com/zeroeightysix/rustbar) - Served as a good demo for writing a basic GTK bar in Rust
