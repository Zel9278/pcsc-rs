{
  description = "PC Status client (pcsc-rs): sends this PC's status to https://pc-stats.eov2.com";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        pcsc-rs = pkgs.callPackage ./nix/package.nix { };
        default = pcsc-rs;
      });

      overlays.default = final: _prev: { pcsc-rs = final.callPackage ./nix/package.nix { }; };

      # services.pcsc-rs for NixOS (systemd), nix-darwin (LaunchDaemon) and home-manager
      # (systemd user service on Linux, LaunchAgent on macOS)
      nixosModules.default = import ./nix/nixos.nix self;
      darwinModules.default = import ./nix/darwin.nix self;
      homeManagerModules.default = import ./nix/home-manager.nix self;

      formatter = forAllSystems (pkgs: pkgs.nixfmt-rfc-style);
    };
}
