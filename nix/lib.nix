# Helpers shared by the modules
{ lib }:
rec {
  # Environment variables for the client, leaving out unset ones
  environment =
    cfg:
    lib.filterAttrs (_: v: v != null) {
      PASS = cfg.pass;
      HOSTNAME = cfg.hostname;
      PCSC_URI = cfg.uri;
      DEV_MODE = if cfg.devMode then "true" else null;
    };

  assertions = name: cfg: [
    {
      assertion = !cfg.enable || (cfg.pass == null) != (cfg.passFile == null);
      message = "${name}: set exactly one of pass or passFile";
    }
  ];

  # launchd has no EnvironmentFile; read passFile in a shell before starting the client
  launchdProgram =
    cfg:
    let
      exe = lib.getExe cfg.package;
    in
    if cfg.passFile == null then
      [ exe ]
    else
      [
        "/bin/sh"
        "-c"
        "set -a; . ${lib.escapeShellArg (toString cfg.passFile)}; exec ${lib.escapeShellArg exe}"
      ];

  label = "io.github.zel9278.pcsc-rs";
}
