#!/bin/sh

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
	libdbusmenu-gtk3-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libpulse-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH} \
	libluajit-5.1-dev${CROSS_DEB_ARCH:+:$CROSS_DEB_ARCH}
