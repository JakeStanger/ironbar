# Ironbar

Ironbar is a customisable and feature-rich bar targeting the Sway compositor, written in Rust. 
It uses GTK3 and gtk-layer-shell.

The bar can be styled to your liking using CSS and hot-loads style changes. 
For information and examples on styling please see the [wiki](https://github.com/JakeStanger/ironbar/wiki).

## Installation

Install with cargo:

```sh
cargo install ironbar
```

Then just run with `ironbar`.

## Configuration

By default, running will get you a blank bar. To start, you will need a configuration file in `.config/ironbar`.
Ironbar supports a range of file formats so pick your favourite:

- JSON
- TOML
- YAML
- [Corn](https://github.com/jakestanger/corn) (Experimental. JSON/Nix like config lang. Supports variables.)

For a full list of modules and their configuration options, please see the [wiki](https://github.com/JakeStanger/ironbar/wiki).

There are two different approaches to configuring the bar:

### Same configuration across all monitors

> If you have a single monitor, or want the same bar to appear across each of your monitors, choose this option.

The top-level object takes any combination of `left`, `center`, and `right`. These each take a list of modules and determine where they are positioned. 

```json
{
  "left": [],
  "center": [],
  "right": []
}
```

### Different configuration across monitors

> If you have multiple monitors and want them to differ in configuration, choose this option.
 
The top-level object takes a single key called `monitors`. This takes an array where each entry is an object with a configuration for each monitor.
The monitor's config object takes any combination of `left`, `center`, and `right`. These each take a list of modules and determine where they are positioned. 

```json
{
  "monitors": [
    {
      "left": [],
      "center": [],
      "right": []
    },
    {
      "left": [],
      "center": [],
      "right": []
    }
  ]
}
```

## Styling

To get started, create a stylesheet at `.config/ironbar/style.css`. Changes will be hot-reloaded every time you save the file.

An example stylesheet and information about each module's styling information can be found on the [wiki](https://github.com/JakeStanger/ironbar/wiki).

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