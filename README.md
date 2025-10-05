<h1 align="center" >--- Ironbar ---</h1> 

<div align="center">
    <a href="https://github.com/JakeStanger/ironbar/releases">
        <img src="https://img.shields.io/crates/v/ironbar?label=version&style=for-the-badge" alt="Current version" />
    </a>
    <a href="https://github.com/JakeStanger/ironbar/actions/workflows/build.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/jakestanger/ironbar/build.yml?style=for-the-badge" alt="Build status" />
    </a>
    <a href="https://github.com/JakeStanger/ironbar/issues">
        <img src="https://img.shields.io/github/issues/jakestanger/ironbar?style=for-the-badge" alt="Open issues" />
    </a>
    <a href="https://github.com/JakeStanger/ironbar/blob/master/LICENSE">
        <img src="https://img.shields.io/github/license/jakestanger/ironbar?style=for-the-badge" alt="License" />
    </a>
    <a href="https://crates.io/crates/ironbar">
        <img src="https://img.shields.io/crates/d/ironbar?label=crates.io%20downloads&style=for-the-badge" alt="Crates.io downloads" />
    </a>
</div>

---

<div align="center">
A customisable and feature-rich GTK4 bar for Wayland compositors, written in Rust.

Ironbar is designed to support anything from a lightweight bar to a full desktop panel with ease.

---

## Getting Started

[Wiki](https://github.com/JakeStanger/ironbar/wiki)
|
[Configuration Guide](https://github.com/JakeStanger/ironbar/wiki/configuration-guide)
|
[Style Guide](https://github.com/JakeStanger/ironbar/wiki/styling-guide)


---

![Screenshot of fully configured bar with MPD widget open](https://f.jstanger.dev/github/ironbar/bar.png?raw)

✨ Looking for a starting point, or want to show off? Head to [Show and tell](https://github.com/JakeStanger/ironbar/discussions/categories/show-and-tell) ✨

</div>

---

## Features

- First-class support for Sway and Hyprland, and partial support for Niri
- Fully themeable with hot-loaded CSS
- Popups to show rich content
- A range of modules which integrate with native desktop libraries
- Ability to create custom widgets, run scripts and embed dynamic content (including via Lua)
- Easy to configure anything from a single bar across all monitors, to multiple different unique bars per monitor 
- Support for multiple config languages

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/ironbar.svg)](https://repology.org/project/ironbar/versions)

Ironbar can be installed from source or using your preferred package manager.

It is also recommended to install a [Nerd Font](https://www.nerdfonts.com/#home) for displaying symbols.

### Cargo

[crate](https://crates.io/crates/ironbar)

Ensure you have the [build dependencies](https://github.com/JakeStanger/ironbar/wiki/compiling#Build-requirements) installed.

```sh
cargo install ironbar --locked
```

### Arch Linux

[aur bin package](https://aur.archlinux.org/packages/ironbar-bin) | [aur git package](https://aur.archlinux.org/packages/ironbar-git)

```sh
yay -S ironbar-git
# or 
yay -S ironbar-bin
```

### Nix

[nix package](https://search.nixos.org/packages?channel=unstable&show=ironbar)

```sh
nix-shell -p ironbar
```

#### Flake

A flake is included with the repo which can be used with Home Manager.

<details>
<summary>Example usage</summary>

```nix
{
  # Add the ironbar flake input
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.ironbar = {
    url = "github:JakeStanger/ironbar";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.hm = {
    url = "github:nix-community/home-manager";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs: {
    homeManagerConfigurations."USER@HOSTNAME" = inputs.hm.lib.homeManagerConfiguration {
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
      modules = [
        # And add the home-manager module
        inputs.ironbar.homeManagerModules.default
        {
          # And configure
          programs.ironbar = {
            enable = true;
            systemd = true;
            config = {
              # An example: 
              monitors = {
                DP-1 = {
                  anchor_to_edges = true;
                  position = "top";
                  height = 16;
                  start = [
                    { type = "clock"; }
                  ];
                  end = [
                    { 
                      type = "tray";
                      icon_size = 16;
                    }
                  ];
                };
              };
            };
            style = /* css */ ''
              /* An example */
              * {
                font-family: Noto Sans Nerd Font, sans-serif;
                font-size: 16px;
                border: none;
                border-radius: 0;
              }
            '';
            package = inputs.ironbar;
            features = ["feature" "another_feature"];
          };
        }
      ];
    };
  };
}
```

</details>

CI builds are automatically cached by Garnix.
You can use their binary cache by following the steps [here](https://garnix.io/docs/caching).

### Fedora

[fedora package](https://copr.fedorainfracloud.org/coprs/victorvintorez/tilingtools/packages/)

``` sh
dnf copr enable victorvintorez/tilingtools
dnf install ironbar
```

### Void Linux

[void package](https://github.com/void-linux/void-packages/tree/master/srcpkgs/ironbar)

```sh
xbps-install ironbar
```

### Source

[repo](https://github.com/jakestanger/ironbar)

Ensure you have the [build dependencies](https://github.com/JakeStanger/ironbar/wiki/compiling#Build-requirements) installed.

```sh
git clone https://github.com/jakestanger/ironbar.git
cd ironbar
cargo build --release
# change path to wherever you want to install
install target/release/ironbar ~/.local/bin/ironbar
```

By default, all features are enabled. 
See [here](https://github.com/JakeStanger/ironbar/wiki/compiling#features) for controlling which features are included.

## Running

Once installed, you will need to create a config and optionally a stylesheet in `.config/ironbar`.
See the [Configuration Guide](https://github.com/JakeStanger/ironbar/wiki/configuration-guide) and [Style Guide](https://github.com/JakeStanger/ironbar/wiki/styling-guide) for full details.

Ironbar can be launched using the `ironbar` binary.

The `IRONBAR_LOG` and `IRONBAR_FILE_LOG` environment variables can be set
to change console and file log verbosity respectively.
You can use any of `error`, `warn`, `info`, `debug` or `trace`.

These default to `IRONBAR_LOG=info` and `IRONBAR_FILE_LOG=warn`.
Note that you cannot increase the file log verbosity above console verbosity.

Log files can be found at `~/.local/share/ironbar/.log`.

## Status

Ironbar is an **alpha** project. 
It is unfinished and subject to constant breaking changes, and will continue that way until the foundation is rock solid.

If you would like to take the risk and help shape development, any bug reports, feature requests and discussion is welcome.

I use Ironbar on my daily driver, so development is active. Features aim to be stable and well documented before being merged.


## Contribution Guidelines

All are welcome, but I ask a few basic things to help make things easier. Please check [here](https://github.com/JakeStanger/ironbar/blob/master/CONTRIBUTING.md) for details.

## Acknowledgements

- [Waybar](https://github.com/Alexays/Waybar) - A lot of the initial inspiration, and a pretty great bar.
- [Rustbar](https://github.com/zeroeightysix/rustbar) - Served as a good demo for writing a basic GTK bar in Rust
- [Smithay Client Toolkit](https://github.com/Smithay/client-toolkit) - Essential in being able to communicate to Wayland
- [gtk-layer-shell](https://github.com/wmww/gtk-layer-shell) - Ironbar and many other projects would be impossible without this
- [Mixxc](https://github.com/Elvyria/Mixxc) - Basis for Ironbar's PulseAudio client code and a cool standalone volume widget.
