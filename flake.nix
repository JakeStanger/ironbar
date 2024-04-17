{
  description = "Nix Flake for ironbar";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, rust-overlay, crane, naersk, ... }:
    let
      inherit (nixpkgs) lib;

      genSystems = lib.genAttrs [ "aarch64-linux" "x86_64-linux" ];

      pkgsFor = system:
        import nixpkgs {
          inherit system;

          overlays = [ self.overlays.default rust-overlay.overlays.default ];
        };

      mkRustToolchain = pkgs:
        pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

    in {
      overlays.default = final: prev:
        let
          rust = mkRustToolchain final;

          craneLib = (crane.mkLib final).overrideToolchain rust;

          naersk' = prev.callPackage naersk {
            cargo = rust;
            rustc = rust;
          };

          rustPlatform = prev.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          };

          props = builtins.fromTOML (builtins.readFile ./Cargo.toml);

          mkDate = longDate:
            (lib.concatStringsSep "-" [
              (builtins.substring 0 4 longDate)
              (builtins.substring 4 2 longDate)
              (builtins.substring 6 2 longDate)
            ]);

          builder = "naersk";
        in {
          ironbar = let
            version = props.package.version + "+date="
              + (mkDate (self.lastModifiedDate or "19700101")) + "_"
              + (self.shortRev or "dirty");
          in if builder == "crane" then
            prev.callPackage ./nix/default.nix {
              inherit version;
              inherit rustPlatform;
              builderName = builder;
              builder = craneLib;
            }
          else if builder == "naersk" then
            prev.callPackage ./nix/default.nix {
              inherit version;
              inherit rustPlatform;
              builderName = builder;
              builder = naersk';
            }
          else
            prev.callPackage ./nix/default.nix {
              inherit version;
              inherit rustPlatform;
              builderName = builder;
            };
        };

      packages = genSystems (system:
        let pkgs = pkgsFor system;
        in (self.overlays.default pkgs pkgs) // {
          default = self.packages.${system}.ironbar;
        });

      apps = genSystems (system:
        let pkgs = pkgsFor system;
        in {
          default = {
            type = "app";
            program = "${pkgs.ironbar}/bin/ironbar";
          };

          ironbar = {
            type = "app";
            program = "${pkgs.ironbar}/bin/ironbar";
          };
        });

      devShells = genSystems (system:
        let
          pkgs = pkgsFor system;
          rust = mkRustToolchain pkgs;

        in {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rust
              rust-analyzer-unwrapped
              gcc
              gtk3
              gtk-layer-shell
              pkg-config
              openssl
              gdk-pixbuf
              glib
              glib-networking
              shared-mime-info
              gnome.adwaita-icon-theme
              hicolor-icon-theme
              gsettings-desktop-schemas
              libxkbcommon
              libpulseaudio
              luajit
              luajitPackages.lgi
            ];

            RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";
          };

        });

      homeManagerModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.ironbar;
          defaultIronbarPackage =
            self.packages.${pkgs.hostPlatform.system}.default;
          jsonFormat = pkgs.formats.json { };
        in {
          options.programs.ironbar = {
            enable = lib.mkEnableOption "ironbar status bar";

            package = lib.mkOption {
              type = with lib.types; package;
              default = defaultIronbarPackage;
              description = "The package for ironbar to use.";
            };

            systemd = lib.mkOption {
              type = lib.types.bool;
              default = pkgs.stdenv.isLinux;
              description = "Whether to enable to systemd service for ironbar.";
            };

            style = lib.mkOption {
              type = lib.types.lines;
              default = "";
              description = "The stylesheet to apply to ironbar.";
            };

            config = lib.mkOption {
              type = jsonFormat.type;
              default = { };
              description = "The config to pass to ironbar.";
            };

            features = lib.mkOption {
              type = lib.types.listOf lib.types.nonEmptyStr;
              default = [ ];
              description = "The features to be used.";
            };

          };
          config = let pkg = cfg.package.override { features = cfg.features; };
          in lib.mkIf cfg.enable {
            home.packages = [ pkg ];

            xdg.configFile = {
              "ironbar/config.json" = lib.mkIf (cfg.config != "") {
                source = jsonFormat.generate "ironbar-config" cfg.config;
              };

              "ironbar/style.css" =
                lib.mkIf (cfg.style != "") { text = cfg.style; };
            };

            systemd.user.services.ironbar = lib.mkIf cfg.systemd {
              Unit = {
                Description = "Systemd service for Ironbar";
                Requires = [ "graphical-session.target" ];
              };

              Service = {
                Type = "simple";
                ExecStart = "${pkg}/bin/ironbar";
              };

              Install.WantedBy = with config.wayland.windowManager; [
                (lib.mkIf hyprland.systemd.enable "hyprland-session.target")
                (lib.mkIf sway.systemd.enable "sway-session.target")
                (lib.mkIf river.systemd.enable "river-session.target")
              ];
            };
          };
        };
    };

  nixConfig = {
    extra-substituters = [ "https://cache.garnix.io" ];
    extra-trusted-public-keys =
      [ "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g=" ];
  };
}
