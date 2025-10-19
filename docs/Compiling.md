You can compile Ironbar from source using `cargo`. 
Just clone the repo and build:

```sh
git clone https://github.com/jakestanger/ironbar.git
cd ironbar
cargo build --locked --release
# change path to wherever you want to install
install target/release/ironbar ~/.local/bin/ironbar
```

It is also recommended to install a [Nerd Font](https://www.nerdfonts.com/#home) for displaying symbols.

## Build requirements

To build from source, you must have GTK (>= 4.10) and GTK Layer Shell installed.
You also need rust; only the latest stable version is supported.

### Docker

A docker image is available which includes all the requirements.

<https://github.com/JakeStanger/ironbar/pkgs/container/ironbar-build>

```shell
docker run -it -v '.:/app' ghcr.io/jakestanger/ironbar-build /bin/bash
$ cd app
$ cargo build
```

### Arch

```shell
pacman -S gtk4 gtk4-layer-shell dbus pkg-config
# for http support
pacman -S openssl
# for volume support
pacman -S libpulse
# for keyboard support
pacman -S libinput
# for lua/cairo support
pacman -S luajit lua51-lgi
```

### Ubuntu/Debian

```shell
apt install build-essential libgtk-4-dev libgtk4-layer-shell-dev libdbus-1-dev
# for http support
apt install libssl-dev
# for volume support
apt install libpulse-dev
# for keyboard support
apt install libinput-dev
# for lua/cairo support
apt install luajit-dev lua-lgi
```

### Fedora

```shell
dnf install gtk4-devel gtk4-layer-shell-devel dbus-devel pkgconf-pkg-config
# for http support
dnf install openssl-devel
# for volume support
dnf install pulseaudio-libs-devel
# for keyboard support
dnf install libinput-devel
# for lua/cairo support
dnf install luajit-devel lua-lgi
```

## Features

By default, all features are enabled for convenience. 
This can result in a significant compile time.
If you know you are not going to need all the features, you can compile with only the features you need.

As of `v0.15.0`, compiling with no features is about 50% faster. 
On a 3800X, it takes about 45 seconds for no features and 90 seconds for all. 
This difference is expected to increase as the bar develops. 

Features containing a `+` can be stacked, for example `config+json` and `config+yaml` could both be enabled.

To build using only specific features, disable default features and pass a comma separated list to `cargo build`:

```shell
cargo build --release --no-default-features \
  --features http,config+json,clock
```

> ⚠ Make sure you enable at least one `config` feature otherwise you will not be able to start the bar!

| Feature             | Description                                                                                                          |
|---------------------|----------------------------------------------------------------------------------------------------------------------|
| **Core**            |                                                                                                                      |
| http                | Enables HTTP features. Currently this includes the ability to load remote images.                                    |
| ipc                 | Enables the IPC server.                                                                                              |
| cli                 | Enables the CLI. Will also enable `ipc`.                                                                             |
| config+all          | Enables support for all configuration languages.                                                                     |
| config+json         | Enables configuration support for JSON.                                                                              |
| config+yaml         | Enables configuration support for YAML.                                                                              |
| config+toml         | Enables configuration support for TOML.                                                                              |
| config+corn         | Enables configuration support for [Corn](https://github.com/jakestanger/corn).                                       |
| config+ron          | Enables configuration support for [Ron](https://github.com/ron-rs/ron).                                              |
| **Modules**         |                                                                                                                      |
| battery             | Enables the `battery` module.                                                                                        |
| bindmode            | Enables the `bindmode` module.                                                                                       |
| bluetooth           | Enables the `bluetooth` module.                                                                                      |
| cairo               | Enables the `cairo` module                                                                                           |
| clipboard           | Enables the `clipboard` module.                                                                                      |
| clock               | Enables the `clock` module.                                                                                          |
| custom              | Enables the `custom` module.                                                                                         |
| focused             | Enables the `focused` module.                                                                                        |
| keyboard            | Enables the `keyboard` module without keyboard layout support.                                                       |
| keyboard+all        | Enables the `keyboard` module with keyboard layout support for all compositors.                                      |
| keyboard+sway       | Enables the `keyboard` module with keyboard layout support for Sway.                                                 |
| keyboard+hyprland   | Enables the `keyboard` module with keyboard layout support for Hyprland.                                             |
| label               | Enables the `label` module.                                                                                          |
| launcher            | Enables the `launcher` module.                                                                                       |
| music+all           | Enables the `music` module with support for all player types.                                                        |
| music+mpris         | Enables the `music` module with MPRIS support.                                                                       |
| music+mpd           | Enables the `music` module with MPD support.                                                                         |
| network_manager     | Enables the `network_manager` module.                                                                                |
| notifications       | Enables the `notiications` module.                                                                                   |
| sys_info            | Enables the `sys_info` module.                                                                                       |
| script              | Enables the `script` module.                                                                                         |
| tray                | Enables the `tray` module.                                                                                           |
| volume              | Enables the `volume` module.                                                                                         |
| workspaces+all      | Enables the `workspaces` module with support for all compositors.                                                    |
| workspaces+sway     | Enables the `workspaces` module with support for Sway.                                                               |
| workspaces+hyprland | Enables the `workspaces` module with support for Hyprland.                                                           |
| workspaces+niri     | Enables the `workspaces` module with support for Niri.                                                               |
| **Other**           |                                                                                                                      |
| extra               | Enables JSON schema support, shell completion support, and the CLI `--print-schema` and `--print-completions` flags. |

## Shell completions

Compiling Ironbar will produce shell completions for bash, zsh and fish; these can be found in `target/completions`.

You can install these as follows:

Bash: 
```shell
install -Dm644 completions/ironbar.bash /usr/share/bash-completion/completions/ironbar
```

Zsh:
```shell
install -Dm644 completions/_ironbar /usr/share/zsh/site-functions/_ironbar
```

Fish:
```shell
install -Dm644 completions/ironbar.fish /usr/share/fish/vendor_completions.d/ironbar.fish
```

## Speeding up compiling

With the full feature set, Ironbar can take a good while to compile. 
There are a couple of tricks which can be used to improve compile times.

## Linker 

Rust versions older than 1.90 use the default GCC `ld` linker. 
By upgrading to `>=1.90`, the `ldd` linker is used instead.
This provides a large increase in compliation speeds.

## Caching

To speed up subsequent rebuilds, Mozilla's [sccache](https://github.com/mozilla/sccache) tool can be used.
This provides a cache of Rust modules which can be re-used when compiling any other crate.

Install the package for your distro, create/modify the `.cargo/config.toml` file inside the project dir,
then add the following:

```toml
[build]
rustc-wrapper = "/usr/bin/sccache"
```

> [!TIP]
> To get the most of out `sccache`, 
> you can add this to `$HOME/.cargo/config.toml` to enable caching for all Cargo builds.

## Codegen Backend

> [!WARNING]
> The Cranelift backend is experimental and requires the use of the nightly compiler.
> It is designed for development builds only.

If working on the Ironbar codebase, you may see some benefit from using the [Cranelift](https://github.com/rust-lang/rustc_codegen_cranelift) compiler backend.
This is known to shave a further few seconds off the compile time (bringing down from 10 to 7-8 on my own hardware).

Firstly install the component:

```shell
rustup component add rustc-codegen-cranelift-preview --toolchain nightly
```

Then create/modify the `.cargo/config.toml` file inside the project dir, and add the following:

```toml
[unstable]
codegen-backend = true

[profile.dev]
codegen-backend = "cranelift"
```
