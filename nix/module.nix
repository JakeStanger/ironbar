self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.ironbar;
  defaultIronbarPackage = self.packages.${pkgs.hostPlatform.system}.default;
  jsonFormat = pkgs.formats.json {};
in {
  options.programs.ironbar = {
    enable = lib.mkEnableOption "ironbar status bar";

    package = lib.mkOption {
      type = with lib.types; package;
      default = defaultIronbarPackage;
      apply = pkg: pkg.override {features = cfg.features;};
      description = "The package for ironbar to use.";
    };

    systemd = lib.mkEnableOption "systemd service for ironbar.";

    style = lib.mkOption {
      type = lib.types.either (lib.types.lines) (lib.types.path);
      default = "";
      description = "The stylesheet to apply to ironbar.";
    };

    config = lib.mkOption {
      type = jsonFormat.type;
      default = {};
      description = "The config to pass to ironbar.";
    };

    features = lib.mkOption {
      type = lib.types.listOf lib.types.nonEmptyStr;
      default = [];
      description = "The features to be used.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [
      cfg.package
    ];

    xdg.configFile = {
      "ironbar/config.json" = lib.mkIf (cfg.config != "") {
        onChange = "${lib.getExe cfg.package} reload";
        source = jsonFormat.generate "ironbar-config" cfg.config;
      };

      "ironbar/style.css" = lib.mkIf (cfg.style != "") (
        if builtins.isPath cfg.style || lib.isStorePath cfg.style
        then {source = cfg.style;}
        else {text = cfg.style;}
      );
    };

    systemd.user.services.ironbar = lib.mkIf cfg.systemd {
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
        ExecReload = "${lib.getExe cfg.package} reload";
        ExecStart = "${lib.getExe cfg.package}";
        KillMode = "mixed";
        Restart = "on-failure";
      };

      Install.WantedBy = [
        config.wayland.systemd.target
        "tray.target"
        (lib.mkIf config.wayland.windowManager.hyprland.enable "hyprland-session.target")
        (lib.mkIf config.wayland.windowManager.sway.enable "sway-session.target")
        (lib.mkIf config.wayland.windowManager.river.enable "river-session.target")
      ];
    };
  };
}
