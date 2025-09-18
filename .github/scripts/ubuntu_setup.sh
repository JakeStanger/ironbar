#!/usr/bin/env bash

GTK4_LAYER_SHELL_VERSION="1.1.1"

apt-get update && apt-get install --assume-yes \
	libssl-dev \
	libgraphene-1.0-dev \
	libgtk-4-dev \
	libinput-dev \
	libdbusmenu-gtk3-dev \
	libpulse-dev \
	libluajit-5.1-dev

# libgtk4-layer-shell deps
apt-get install --assume-yes \
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

# GH CLI, required by some CI jobs
(type -p wget >/dev/null || (apt update && apt install wget -y)) \
	&& mkdir -p -m 755 /etc/apt/keyrings \
	&& out=$(mktemp) && wget -nv -O$out https://cli.github.com/packages/githubcli-archive-keyring.gpg \
	&& cat "$out" | tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
	&& chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
	&& mkdir -p -m 755 /etc/apt/sources.list.d \
	&& echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
	&& apt update \
	&& apt install gh -y