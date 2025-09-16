#!/usr/bin/env bash

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

# CROSS_DEB_ARCH is empty for native builds
$SUDO apt-get update && $SUDO apt-get install --assume-yes \
	libssl-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libgtk-3-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libgtk-layer-shell-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libinput-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libdbusmenu-gtk3-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libpulse-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libluajit-5.1-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH}

# GH CLI, required by some CI jobs
(type -p wget >/dev/null || ($SUDO apt update && $SUDO apt install wget -y)) \
	&& $SUDO mkdir -p -m 755 /etc/apt/keyrings \
	&& out=$(mktemp) && wget -nv -O$out https://cli.github.com/packages/githubcli-archive-keyring.gpg \
	&& cat "$out" | $SUDO tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
	&& $SUDO chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
	&& $SUDO mkdir -p -m 755 /etc/apt/sources.list.d \
	&& echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | $SUDO tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
	&& $SUDO apt update \
	&& $SUDO apt install gh -y