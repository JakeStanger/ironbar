{
  description = "Nix Flake for ironbar";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-compat.url = "github:edolstra/flake-compat";
    crane.url = "github:ipetkov/crane";
    nix-systems.url = "github:nix-systems/default-linux";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      nix-systems,
      ...
    }:
    let
      forAllSystems =
        function:
        nixpkgs.lib.genAttrs (import nix-systems) (system: function nixpkgs.legacyPackages.${system});
    in
    {
      # Devshell
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            rustfmt
            rust-analyzer
            sccache
            dbus
          ];

          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.ironbar ];
        };

      });

      # Packages
      packages = forAllSystems (pkgs: {
        ironbar =
          let
            props = builtins.fromTOML (builtins.readFile ./Cargo.toml);
            version = props.package.version;
            craneLib = crane.mkLib pkgs;
          in
          pkgs.callPackage ./nix/package.nix {
            inherit version craneLib;
          };

        default = self.packages.${pkgs.stdenv.hostPlatform.system}.ironbar;
      });

      # Apps
      apps = forAllSystems (
        pkgs:
        let
          ironbar = {
            type = "app";
            program = pkgs.lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.ironbar;
          };
        in
        {
          inherit ironbar;
          default = ironbar;
        }
      );

      homeManagerModules.default = import ./nix/module.nix self;
    };

  nixConfig = {
    extra-substituters = [ "https://jakestanger.cachix.org" ];
    extra-trusted-public-keys = [ "jakestanger.cachix.org-1:VWJE7AWNe5/KOEvCQRxoE8UsI2Xs2nHULJ7TEjYm7mM=" ];
  };
}
