#!/usr/bin/env bash

GTK4_LAYER_SHELL_VERSION="1.1.1"

# sudo needed for github runner, not available by default for cross images
if command -v sudo >/dev/null 2>&1; then
	SUDO="sudo"
else
	SUDO=""
fi

# Needed for cross-compilation
if [ -n "$CROSS_DEB_ARCH" ]; then
	$SUDO dpkg --add-architecture "$CROSS_DEB_ARCH"
fi

# Ironbar deps
# CROSS_DEB_ARCH is empty for native builds
$SUDO apt-get update && $SUDO apt-get install --assume-yes \
	libssl-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libgraphene-1.0-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libgtk-4-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libinput-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libdbusmenu-gtk3-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libpulse-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libluajit-5.1-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH}

#	libgtk4-layer-shell-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \

# libgtk4-layer-shell deps
$SUDO apt-get install --assume-yes \
  meson${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
  ninja-build${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
  libwayland-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
  wayland-protocols${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
  gobject-introspection${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
  libgirepository1.0-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
  gtk-doc-tools${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH}  \
  python3${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
  valac${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH}

wget https://github.com/wmww/gtk4-layer-shell/archive/refs/tags/v$GTK4_LAYER_SHELL_VERSION.tar.gz -O /tmp/gtk4-layer-shell.tar.gz
tar -xzf /tmp/gtk4-layer-shell.tar.gz -C /tmp
ls /tmp
pushd /tmp/gtk4-layer-shell-$GTK4_LAYER_SHELL_VERSION || exit 1

meson setup -Dexamples=true -Ddocs=true -Dtests=true build
ninja -C build
ninja -C build install
ldconfig

popd || exit 1
rm -rf /tmp/gtk4-layer-shell.tar.gz /tmp/gtk4-layer-shell-$GTK4_LAYER_SHELL_VERSION