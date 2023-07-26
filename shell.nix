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
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];
}