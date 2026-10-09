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

    # A fixed user, not DynamicUser: DynamicUser makes the whole file system read-only for the
    # service, and the client leaves read-only mounts out, so no disk would be listed
    users.users.pcsc-rs = {
      isSystemUser = true;
      group = "pcsc-rs";
      description = "PC Status client";
    };
    users.groups.pcsc-rs = { };

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
        User = "pcsc-rs";
        Group = "pcsc-rs";
        NoNewPrivileges = true;
      };
    };
  };
}
