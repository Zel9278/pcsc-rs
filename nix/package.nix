{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  openssl,
  zlib,
}:
let
  cargoToml = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "pcsc-rs";
  inherit (cargoToml.package) version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../build.rs
      ../src
      ../assets
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ zlib ] ++ lib.optionals stdenv.hostPlatform.isLinux [ openssl ];
  # Cargo.toml asks for a vendored OpenSSL so release builds are static; use nixpkgs' instead
  env.OPENSSL_NO_VENDOR = "1";

  meta = {
    description = "PC Status client: sends this PC's status to https://pc-stats.eov2.com";
    homepage = "https://github.com/Zel9278/pcsc-rs";
    mainProgram = "pcsc-rs";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
