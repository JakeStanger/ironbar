{
  gtk3,
  gdk-pixbuf,
  gtk-layer-shell,
  libxkbcommon,
  openssl,
  pkg-config,
  rustPlatform,
  lib,
  version ? "git",
  features ? [],
}:
rustPlatform.buildRustPackage {
  inherit version;
  pname = "ironbar";
  src = builtins.path {
    name = "ironbar";
    path = lib.cleanSource ../.;
  };
  buildNoDefaultFeatures =
    if features == []
    then false
    else true;
  buildFeatures = features;
  cargoDeps = rustPlatform.importCargoLock {lockFile = ../Cargo.lock;};
  cargoLock.lockFile = ../Cargo.lock;
  nativeBuildInputs = [pkg-config];
  buildInputs = [gtk3 gdk-pixbuf gtk-layer-shell libxkbcommon openssl];
  meta = with lib; {
    homepage = "https://github.com/JakeStanger/ironbar";
    description = "Customisable gtk-layer-shell wlroots/sway bar written in rust.";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "ironbar";
  };
}
