You can compile Ironbar from source using `cargo`. 
Just clone the repo and build:

```sh
git clone https://github.com/jakestanger/ironbar.git
cd ironbar
cargo build --release
# change path to wherever you want to install
install target/release/ironbar ~/.local/bin/ironbar
```

## Build requirements

To build from source, you must have GTK (>= 3.22) and GTK Layer Shell installed.
You also need rust; only the latest stable version is supported.

### Arch

```shell
pacman -S gtk3 gtk-layer-shell
# for http support
pacman -S openssl
# for volume support
pacman -S libpulse
```

### Ubuntu/Debian

```shell
apt install build-essential libgtk-3-dev libgtk-layer-shell-dev
# for http support
apt install libssl-dev
# for volume support
apt install libpulse-dev
```

### Fedora

```shell
dnf install gtk3-devel gtk-layer-shell-devel
# for http support
dnf install openssl-devel
# for volume support
dnf install libpulseaudio-devel
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
| ipc                 | Enables the IPC server.                                                           |
| cli                 | Enables the CLI. Will also enable `ipc`.                                          |
| config+all          | Enables support for all configuration languages.                                  |
| config+json         | Enables configuration support for JSON.                                           |
| config+yaml         | Enables configuration support for YAML.                                           |
| config+toml         | Enables configuration support for TOML.                                           |
| config+corn         | Enables configuration support for [Corn](https://github.com/jakestanger/corn).    |
| config+ron          | Enables configuration support for [Ron](https://github.com/ron-rs/ron).           |
| **Modules**         |                                                                                   |
| clipboard           | Enables the `clipboard` module.                                                   |
| clock               | Enables the `clock` module.                                                       |
| focused             | Enables the `focused` module.                                                     |
| launcher            | Enables the `launcher` module.                                                    |
| music+all           | Enables the `music` module with support for all player types.                     |
| music+mpris         | Enables the `music` module with MPRIS support.                                    |
| music+mpd           | Enables the `music` module with MPD support.                                      |
| sys_info            | Enables the `sys_info` module.                                                    |
| tray                | Enables the `tray` module.                                                        |
| upower              | Enables the `upower` module.                                                      |
| volume              | Enables the `volume` module.                                                      |
| workspaces+all      | Enables the `workspaces` module with support for all compositors.                 |
| workspaces+sway     | Enables the `workspaces` module with support for Sway.                            |
| workspaces+hyprland | Enables the `workspaces` module with support for Hyprland.                        |

## Speeding up compiling

With the full feature set, Ironbar can take a good while to compile. 
There are a couple of tricks which can be used to improve compile times.

## Linker 

The default GCC linker is *slow* - it takes nearly half of the compile time.
As an alternative, you can use [mold](https://github.com/rui314/mold).

Install the package for your distro, create/modify the `.cargo/config.toml` file inside the project dir,
then add the following:

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

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