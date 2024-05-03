{
  gtk3,
  gdk-pixbuf,
  librsvg,
  webp-pixbuf-loader,
  gobject-introspection,
  glib-networking,
  glib,
  shared-mime-info,
  gsettings-desktop-schemas,
  wrapGAppsHook,
  gtk-layer-shell,
  gnome,
  libxkbcommon,
  libpulseaudio,
  openssl,
  luajit,
  luajitPackages,
  pkg-config,
  hicolor-icon-theme,
  rustPlatform,
  lib,
  version ? "git",
  features ? [],
  builderName ? "nix",
  builder ? {},
}: let
  hasFeature = f: features == [ ] || builtins.elem f features;

  basePkg = rec {
    inherit version;

    pname = "ironbar";

    src = builtins.path {
      name = "ironbar";
      path = lib.cleanSource ../.;
    };

    nativeBuildInputs = [
        pkg-config
        wrapGAppsHook
        gobject-introspection
    ];

    buildInputs = [
      gtk3
      gdk-pixbuf
      glib
      gtk-layer-shell
      glib-networking
      shared-mime-info
      gnome.adwaita-icon-theme
      hicolor-icon-theme
      gsettings-desktop-schemas
      libxkbcommon ]
      ++ (if hasFeature "http" then [ openssl ] else [])
      ++ (if hasFeature "volume" then [ libpulseaudio ] else [])
      ++ (if hasFeature "cairo" then [ luajit ] else []);

    propagatedBuildInputs = [ gtk3 ];

    lgi = luajitPackages.lgi;

    gappsWrapperArgs = ''
        # Thumbnailers
            --prefix XDG_DATA_DIRS : "${gdk-pixbuf}/share"
            --prefix XDG_DATA_DIRS : "${librsvg}/share"
            --prefix XDG_DATA_DIRS : "${webp-pixbuf-loader}/share"
            --prefix XDG_DATA_DIRS : "${shared-mime-info}/share"

            # gtk-launch
            --suffix PATH : "${lib.makeBinPath [ gtk3 ]}"
    ''
    + (if hasFeature "cairo" then ''
        --prefix LUA_PATH : "./?.lua;${lgi}/share/lua/5.1/?.lua;${lgi}/share/lua/5.1/?/init.lua;${luajit}/share/lua/5.1/\?.lua;${luajit}/share/lua/5.1/?/init.lua"
        --prefix LUA_CPATH : "./?.so;${lgi}/lib/lua/5.1/?.so;${luajit}/lib/lua/5.1/?.so;${luajit}/lib/lua/5.1/loadall.so"
    '' else "");

    preFixup = ''
      gappsWrapperArgs+=(
        ${gappsWrapperArgs}
      )
    '';

    passthru = {
      updateScript = gnome.updateScript {
        packageName = pname;
        attrPath = "gnome.${pname}";
      };
    };

    meta = with lib; {
      homepage = "https://github.com/JakeStanger/ironbar";
      description =
        "Customisable gtk-layer-shell wlroots/sway bar written in rust.";
      license = licenses.mit;
      platforms = platforms.linux;
      mainProgram = "ironbar";
    };

  };

  flags = let
    noDefault = if features == [ ] then "" else "--no-default-features";

    featuresStr = if features == [ ] then
      ""
    else
      ''-F "${builtins.concatStringsSep "," features}"'';

  in [ noDefault featuresStr ];
in if builderName == "naersk" then
  builder.buildPackage (basePkg // { cargoBuildOptions = old: old ++ flags; })
else if builderName == "crane" then
  builder.buildPackage (basePkg // {
    cargoExtraArgs = builtins.concatStringsSep " " flags;
    doCheck = false;
  })
else
  rustPlatform.buildRustPackage (basePkg // {
    buildNoDefaultFeatures = features != [ ];

    buildFeatures = features;
    cargoDeps = rustPlatform.importCargoLock { lockFile = ../Cargo.lock; };
    cargoLock.lockFile = ../Cargo.lock;
    cargoLock.outputHashes."stray-0.1.3" =
      "sha256-7mvsWZFmPWti9AiX67h6ZlWiVVRZRWIxq3pVaviOUtc=";
  })
