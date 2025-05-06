{
  description = "Nix Flake for ironbar";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-compat.url = "github:edolstra/flake-compat";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    rust-overlay,
    crane,
    naersk,
    flake-parts,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} (let
      mkRustToolchain = pkgs:
        pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src"];
        };
    in {
      systems = [
        "aarch64-linux"
        "x86_64-linux"
      ];

      imports = [
        ./nix/devshell.nix
      ];

      perSystem = {
        system,
        config,
        pkgs,
        lib,
        ...
      }: {
        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [
            self.overlays.default
            rust-overlay.overlays.default
          ];
        };

        # Packages
        packages = {
          ironbar = pkgs.ironbar;
          default = pkgs.ironbar;
        };

        # Apps
        apps = {
          ironbar = {
            type = "app";
            program = lib.getExe pkgs.ironbar;
          };
          default = config.apps.ironbar;
        };
      };

      flake = {
        overlays.default = final: prev: let
          inherit (nixpkgs) lib;

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

          mkDate = longDate: (lib.concatStringsSep "-" [
            (builtins.substring 0 4 longDate)
            (builtins.substring 4 2 longDate)
            (builtins.substring 6 2 longDate)
          ]);

          builder = "naersk";
        in {
          ironbar = let
            version =
              props.package.version
              + "+date="
              + (mkDate (self.lastModifiedDate or "19700101"))
              + "_"
              + (self.shortRev or "dirty");
          in
            if builder == "crane"
            then
              prev.callPackage ./nix/package.nix {
                inherit version;
                inherit rustPlatform;
                builderName = builder;
                builder = craneLib;
              }
            else if builder == "naersk"
            then
              prev.callPackage ./nix/package.nix {
                inherit version;
                inherit rustPlatform;
                builderName = builder;
                builder = naersk';
              }
            else
              prev.callPackage ./nix/package.nix {
                inherit version;
                inherit rustPlatform;
                builderName = builder;
              };
        };

        homeManagerModules.default = import ./nix/module.nix self;
      };
    });

  nixConfig = {
    extra-substituters = ["https://cache.garnix.io"];
    extra-trusted-public-keys = ["cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="];
  };
}
