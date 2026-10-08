# nix-darwin: a LaunchDaemon for the whole machine
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
in
{
  options.services.pcsc-rs = import ./options.nix self { inherit lib pkgs; };

  config = lib.mkIf cfg.enable {
    assertions = helpers.assertions "services.pcsc-rs" cfg;

    launchd.daemons.pcsc-rs.serviceConfig = {
      Label = helpers.label;
      ProgramArguments = helpers.launchdProgram cfg;
      EnvironmentVariables = helpers.environment cfg;
      RunAtLoad = true;
      KeepAlive = true;
      ThrottleInterval = 10;
      StandardOutPath = "/var/log/pcsc-rs.log";
      StandardErrorPath = "/var/log/pcsc-rs.log";
    };
  };
}
