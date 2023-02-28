{
  description = "Nix Flake for ironbar";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    #nci.url = "github:yusdacra/nix-cargo-integration";
    #nci.inputs.nixpkgs.follows = "nixpkgs";
    #nci.inputs.rust-overlay.follows = "rust-overlay";
  };
  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    ...
  }: let
    inherit (nixpkgs) lib;
    genSystems = lib.genAttrs [
      "aarch64-linux"
      "x86_64-linux"
    ];
    pkgsFor = system:
      import nixpkgs {
        inherit system;

        overlays = [
          self.overlays.default
          rust-overlay.overlays.default
        ];
      };
    mkRustToolchain = pkgs: pkgs.rust-bin.stable.latest.default;
    # defaultFeatures = [
    #   "http"
    #   "config+all"
    #   "clock"
    #   "music+all"
    #   "sys_info"
    #   "tray"
    #   "workspaces+all"
    # ];
  in {
    overlays.default = final: prev: let
      rust = mkRustToolchain final;

      rustPlatform = prev.makeRustPlatform {
        cargo = rust;
        rustc = rust;
      };
    in {
      ironbar = features:
        rustPlatform.buildRustPackage {
          pname = "ironbar";
          version = self.rev or "dirty";
          src = builtins.path {
            name = "ironbar";
            path = prev.lib.cleanSource ./.;
          };
          buildNoDefaultFeatures = if features == [] then false else true;
          buildFeatures = features;
          cargoDeps = rustPlatform.importCargoLock {lockFile = ./Cargo.lock;};
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with prev; [pkg-config];
          buildInputs = with prev; [gtk3 gdk-pixbuf gtk-layer-shell libxkbcommon openssl];
        };
    };
    packageBuilder = genSystems (system: self.packages.${system}.ironbar);
    packages = genSystems (
      system: let
        pkgs = pkgsFor system;
      in
        (self.overlays.default pkgs pkgs)
        // {
          default = self.packages.${system}.ironbar [];
        }
    );
    devShells = genSystems (system: let
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
        ];

        RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";
      };
    });
    homeManagerModules.default = {
      config,
      lib,
      pkgs,
      ...
    }: let
      cfg = config.programs.ironbar;
      defaultIronbarPackage = self.packages.${pkgs.hostPlatform.system}.default [];
      jsonFormat = pkgs.formats.json {};
    in {
      options.programs.ironbar = {
        enable = lib.mkEnableOption "ironbar status bar";
        package = lib.mkOption {
          type = with lib.types; package;
          default = defaultIronbarPackage;
          description = "The package for ironbar to use";
        };
        systemd = lib.mkOption {
          type = lib.types.bool;
          default = pkgs.stdenv.isLinux;
          description = "Whether to enable to systemd service for ironbar";
        };
        style = lib.mkOption {
          type = lib.types.lines;
          default = "";
          description = "The stylesheet to apply to ironbar";
        };
        config = lib.mkOption {
          type = jsonFormat.type;
          default = {};
          description = "The config to pass to ironbar";
        };
      };
      config = lib.mkIf cfg.enable {
        home.packages = [cfg.package];
        xdg.configFile = {
          "ironbar/config.json" = lib.mkIf (cfg.config != "") {
            source = jsonFormat.generate "ironbar-config" cfg.config;
          };
          "ironbar/style.css" = lib.mkIf (cfg.style != "") {
            text = cfg.style;
          };
        };
        systemd.user.services.ironbar = lib.mkIf cfg.systemd {
          Unit = {
            Description = "Systemd service for Ironbar";
            Requires = ["graphical-session.target"];
          };
          Service = {
            Type = "simple";
            ExecStart = "${cfg.package}/bin/ironbar";
          };
          Install.WantedBy = [
            (lib.mkIf config.wayland.windowManager.hyprland.systemdIntegration "hyprland-session.target")
            (lib.mkIf config.wayland.windowManager.sway.systemdIntegration "sway-session.target")
          ];
        };
      };
    };
  };
}
