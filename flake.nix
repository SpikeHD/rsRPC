{
  description = "Alternative Discord RPC server CLI tool and Rust library, inspired by arRPC.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};
      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            rustPlatform.bindgenHook
          ];

          buildInputs = with pkgs; [
            openssl
          ];
        };
      }
    );
}
