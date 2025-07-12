{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  # Get dependencies from the main package
  inputsFrom = [(pkgs.callPackage ./default.nix {})];
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
