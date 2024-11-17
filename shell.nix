{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    clippy
    rustfmt
    gtk3
    gtk-layer-shell
    gcc
    openssl
    libdbusmenu-gtk3
    libpulseaudio
    libinput
    libevdev
    luajit
    luajitPackages.lgi
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];
}