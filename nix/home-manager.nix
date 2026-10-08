# home-manager: a systemd user service on Linux, a LaunchAgent on macOS.
# For Nix on other Linux distributions and on macOS without nix-darwin.
self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.pcsc-rs;
  helpers = import ./lib.nix { inherit lib; };
  env = helpers.environment cfg;
in
{
  options.services.pcsc-rs = import ./options.nix self { inherit lib pkgs; };

  config = lib.mkIf cfg.enable (
    lib.mkMerge [
      { assertions = helpers.assertions "services.pcsc-rs" cfg; }

      (lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
        systemd.user.services.pcsc-rs = {
          Unit = {
            Description = "PC Status client";
            After = [ "network-online.target" ];
          };
          Service = {
            ExecStart = lib.getExe cfg.package;
            Environment = lib.mapAttrsToList (k: v: "${k}=${v}") env;
            EnvironmentFile = lib.mkIf (cfg.passFile != null) (toString cfg.passFile);
            Restart = "always";
            RestartSec = 5;
          };
          Install.WantedBy = [ "default.target" ];
        };
      })

      (lib.mkIf pkgs.stdenv.hostPlatform.isDarwin {
        launchd.agents.pcsc-rs = {
          enable = true;
          config = {
            Label = helpers.label;
            ProgramArguments = helpers.launchdProgram cfg;
            EnvironmentVariables = env;
            RunAtLoad = true;
            KeepAlive = true;
            ThrottleInterval = 10;
            StandardOutPath = "${config.home.homeDirectory}/Library/Logs/pcsc-rs.log";
            StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/pcsc-rs.log";
          };
        };
      })
    ]
  );
}
