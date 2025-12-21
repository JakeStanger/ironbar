{
  description = "Nix Flake for ironbar";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-compat.url = "github:edolstra/flake-compat";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-systems.url = "github:nix-systems/default-linux";
  };

  outputs = {
    self,
    nixpkgs,
    naersk,
    nix-systems,
    ...
  }: let
    forAllSystems = function:
      nixpkgs.lib.genAttrs (import nix-systems) (system: function nixpkgs.legacyPackages.${system});
    mkDate = longDate: (nixpkgs.lib.concatStringsSep "-" [
      (builtins.substring 0 4 longDate)
      (builtins.substring 4 2 longDate)
      (builtins.substring 6 2 longDate)
    ]);
  in {
    # Devshell
    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = builtins.attrValues {
          inherit
            (pkgs)
            cargo
            clippy
            rustfmt
            dbus
            gtk4
            gtk4-layer-shell
            gcc
            openssl
            libpulseaudio
            libinput
            libevdev
            luajit
            sccache
            ;
          inherit (pkgs.luajitPackages) lgi;
        };

        nativeBuildInputs = [
          pkgs.pkg-config
        ];
      };
    });

    # Packages
    packages = forAllSystems (pkgs: {
      ironbar = let
        props = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        version =
          props.package.version
          + "+date="
          + (mkDate (self.lastModifiedDate or "19700101"))
          + "_"
          + (self.shortRev or "dirty");
        naersk' = pkgs.callPackage naersk {};
      in
        pkgs.callPackage ./nix/package.nix {
          inherit version;
          naersk = naersk';
        };

      default = self.packages.${pkgs.hostPlatform.system}.ironbar;
    });

    # Apps
    apps = forAllSystems (pkgs: let
      ironbar = {
        type = "app";
        program = pkgs.lib.getExe self.packages.${pkgs.hostPlatform.system}.ironbar;
      };
    in {
      inherit ironbar;
      default = ironbar;
    });

    homeManagerModules.default = import ./nix/module.nix self;
  };

  nixConfig = {
    extra-substituters = ["https://cache.garnix.io"];
    extra-trusted-public-keys = ["cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="];
  };
}
