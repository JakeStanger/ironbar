{
  gtk4,
  gdk-pixbuf,
  librsvg,
  webp-pixbuf-loader,
  gobject-introspection,
  glib-networking,
  glib,
  shared-mime-info,
  gsettings-desktop-schemas,
  wrapGAppsHook4,
  gtk4-layer-shell,
  gnome,
  libxkbcommon,
  libpulseaudio,
  libinput,
  libevdev,
  openssl,
  luajit,
  luajitPackages,
  pkg-config,
  installShellFiles,
  adwaita-icon-theme,
  hicolor-icon-theme,
  lib,
  version ? "git",
  features ? [],
  naersk,
  dbus,
}: let
  hasFeature = f: features == [] || builtins.elem f features;
  flags = let
    noDefault =
      if features == []
      then ""
      else "--no-default-features";

    featuresStr =
      if features == []
      then ""
      else ''-F "${builtins.concatStringsSep "," features}"'';
  in [
    noDefault
    featuresStr
  ];
  lgi = luajitPackages.lgi;
  gappsWrapperArgs =
    ''
      # Thumbnailers
          --prefix XDG_DATA_DIRS : "${gdk-pixbuf}/share"
          --prefix XDG_DATA_DIRS : "${librsvg}/share"
          --prefix XDG_DATA_DIRS : "${webp-pixbuf-loader}/share"
          --prefix XDG_DATA_DIRS : "${shared-mime-info}/share"

          # gtk-launch
          --suffix PATH : "${lib.makeBinPath [gtk4]}"
    ''
    + lib.optionalString (hasFeature "cairo") ''
      --prefix LUA_PATH : "./?.lua;${lgi}/share/lua/5.1/?.lua;${lgi}/share/lua/5.1/?/init.lua;${luajit}/share/lua/5.1/\?.lua;${luajit}/share/lua/5.1/?/init.lua"
      --prefix LUA_CPATH : "./?.so;${lgi}/lib/lua/5.1/?.so;${luajit}/lib/lua/5.1/?.so;${luajit}/lib/lua/5.1/loadall.so"
    '';
in
  naersk.buildPackage {
    inherit version;

    pname = "ironbar";

    src = let
      fs = lib.fileset;
      root = ../.;
      nixRelated = fs.fileFilter (file: file.hasExt "nix" || file.name == "flake.lock") root;
      cicdRelated = fs.unions [
        (lib.path.append root "Dockerfile")
        (lib.path.append root ".github")
      ];
      ideRelated = fs.unions [
        (lib.path.append root ".idea")
      ];
    in
      fs.toSource {
        inherit root;
        # NOTE: can possibly filter out more
        fileset = fs.difference root (
          fs.unions [
            nixRelated
            cicdRelated
            ideRelated
          ]
        );
      };

    nativeBuildInputs = [
      pkg-config
      wrapGAppsHook4
      gobject-introspection
      installShellFiles
    ];

    buildInputs =
      [
        gtk4
        gdk-pixbuf
        glib
        gtk4-layer-shell
        glib-networking
        shared-mime-info
        adwaita-icon-theme
        hicolor-icon-theme
        gsettings-desktop-schemas
        libxkbcommon
        dbus
      ]
      ++ lib.optionals (hasFeature "http") [openssl]
      ++ lib.optionals (hasFeature "volume") [libpulseaudio]
      ++ lib.optionals (hasFeature "cairo") [luajit]
      ++ lib.optionals (hasFeature "keyboard") [
        libinput
        libevdev
      ];

    propagatedBuildInputs = [gtk4];

    cargoBuildOptions = old: old ++ flags;

    preFixup = ''
      gappsWrapperArgs+=(
        ${gappsWrapperArgs}
      )
    '';

    postInstall = ''
      mkdir -p target/completions
      target/release/ironbar --print-completions bash > target/completions/ironbar.bash
      target/release/ironbar --print-completions zsh > target/completions/_ironbar
      target/release/ironbar --print-completions fish > target/completions/ironbar.fish

      installShellCompletion --cmd ironbar \
        --bash target/completions/ironbar.bash \
        --fish target/completions/ironbar.fish \
        --zsh target/completions/_ironbar
    '';

    passthru = {
      updateScript = gnome.updateScript {
        packageName = "ironbar";
        attrPath = "gnome.ironbar";
      };
    };

    meta = {
      homepage = "https://github.com/JakeStanger/ironbar";
      description = "Customisable gtk-layer-shell wlroots/sway bar written in rust.";
      license = lib.licenses.mit;
      platforms = lib.platforms.linux;
      mainProgram = "ironbar";
    };
  }
