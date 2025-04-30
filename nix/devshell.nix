{
  perSystem = { pkgs, ... }: {
    devShells.default = pkgs.mkShell {
      packages = with pkgs; [
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
    };
  };
}
