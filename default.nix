{pkgs ? import <nixpkgs> {}}: let
  manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in
  pkgs.rustPlatform.buildRustPackage rec {
    pname = manifest.name;
    version = manifest.version;
    cargoLock.lockFile = ./Cargo.lock;
    src = pkgs.lib.cleanSource ./.;

    # Additional tooling
    nativeBuildInputs = with pkgs; [
      pkg-config
      rustc
      cargo
      rust-analyzer
    ];
    buildInputs = with pkgs; [
      openssl
    ];
  }
