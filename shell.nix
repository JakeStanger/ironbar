{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    clippy
    rustfmt
    gtk4
    gtk4-layer-shell
    gcc
    openssl
    # libdbusmenu-gtk3
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
