self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.ironbar;
  defaultIronbarPackage = self.packages.${pkgs.hostPlatform.system}.default;
  jsonFormat = pkgs.formats.json {};
  inherit
    (lib)
    types
    mkOption
    mkEnableOption
    mkIf
    getExe
    ;
in {
  options.programs.ironbar = {
    enable = mkEnableOption "ironbar status bar";

    package = mkOption {
      type = types.package;
      default = defaultIronbarPackage;
      apply = pkg: pkg.override {features = cfg.features;};
      description = "The package for ironbar to use.";
    };

    systemd = mkEnableOption "systemd service for ironbar.";

    style = mkOption {
      type = types.either (types.lines) (types.path);
      default = "";
      description = "The stylesheet to apply to ironbar.";
    };

    config = mkOption {
      type = jsonFormat.type;
      default = {};
      description = "The config to pass to ironbar.";
    };

    features = mkOption {
      type = types.listOf types.nonEmptyStr;
      default = [];
      description = "The features to be used.";
    };
  };

  config = mkIf cfg.enable {
    home.packages = [
      cfg.package
    ];

    xdg.configFile = {
      "ironbar/config.json" = mkIf (cfg.config != "") {
        onChange = "${getExe cfg.package} reload";
        source = jsonFormat.generate "ironbar-config" cfg.config;
      };

      "ironbar/style.css" = mkIf (cfg.style != "") (
        if builtins.isPath cfg.style || lib.isStorePath cfg.style
        then {source = cfg.style;}
        else {text = cfg.style;}
      );
    };

    systemd.user.services.ironbar = mkIf cfg.systemd {
      Unit = {
        Description = "Systemd service for Ironbar";
        Documentation = "https://github.com/JakeStanger/ironbar";
        PartOf = [
          config.wayland.systemd.target
          "tray.target"
        ];
        After = [config.wayland.systemd.target];
        ConditionEnvironment = "WAYLAND_DISPLAY";
      };

      Service = {
        ExecReload = "${getExe cfg.package} reload";
        ExecStart = "${getExe cfg.package}";
        KillMode = "mixed";
        Restart = "on-failure";
      };

      Install.WantedBy = [
        config.wayland.systemd.target
        "tray.target"
        (mkIf config.wayland.windowManager.hyprland.enable "hyprland-session.target")
        (mkIf config.wayland.windowManager.sway.enable "sway-session.target")
        (mkIf config.wayland.windowManager.river.enable "river-session.target")
      ];
    };
  };
}
