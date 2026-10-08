# Options shared by the NixOS, nix-darwin and home-manager modules
self:
{ lib, pkgs }:
let
  inherit (lib) mkOption types;
in
{
  enable = lib.mkEnableOption "the PC Status client (pcsc-rs)";

  package = mkOption {
    type = types.package;
    default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
    defaultText = lib.literalExpression "pcsc-rs.packages.\${system}.default";
    description = "The pcsc-rs package. Nix keeps it up to date; the client does not update itself.";
  };

  pass = mkOption {
    type = types.nullOr types.str;
    default = null;
    description = ''
      Password shared with the server (PASS). It ends up in the world-readable
      Nix store; use passFile to keep it out.
    '';
  };

  passFile = mkOption {
    type = types.nullOr types.path;
    default = null;
    example = "/run/secrets/pcsc-rs.env";
    description = "File containing a line PASS=<password>, read when the client starts.";
  };

  hostname = mkOption {
    type = types.nullOr types.str;
    default = null;
    description = "Name shown on PC Status (HOSTNAME). The system's hostname when null.";
  };

  uri = mkOption {
    type = types.nullOr types.str;
    default = null;
    example = "wss://pcss.eov2.com/server";
    description = "Server to connect to (PCSC_URI). The client's default when null.";
  };

  devMode = mkOption {
    type = types.bool;
    default = false;
    description = "Let several clients share a hostname (DEV_MODE=true).";
  };
}
