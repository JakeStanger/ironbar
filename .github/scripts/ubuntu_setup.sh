#!/usr/bin/env bash

apt-get update && apt-get install --assume-yes \
	libssl-dev \
	libgraphene-1.0-dev \
	libgtk-4-dev \
	libinput-dev \
	libdbusmenu-gtk3-dev \
	libpulse-dev \
	libluajit-5.1-dev \
	libgtk4-layer-shell-dev

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