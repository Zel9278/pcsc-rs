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

    systemd.services.pcsc-rs = {
      description = "PC Status client";
      wantedBy = [ "multi-user.target" ];
      wants = [ "network-online.target" ];
      after = [ "network-online.target" ];
      # nvidia-smi, when the NVIDIA driver is installed
      path = [ "/run/current-system/sw" ];
      environment = helpers.environment cfg;
      serviceConfig = {
        ExecStart = lib.getExe cfg.package;
        EnvironmentFile = lib.mkIf (cfg.passFile != null) cfg.passFile;
        Restart = "always";
        RestartSec = 5;
        # Only reads /proc, /sys and disk usage; no need for root
        DynamicUser = true;
        NoNewPrivileges = true;
        ProtectHome = "read-only";
      };
    };
  };
}
