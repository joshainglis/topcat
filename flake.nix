{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      flake-utils,
      rust-overlay,
      nixpkgs,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
          config.allowUnfree = true;
        };

        rustToolchain = pkgs.rust-bin.stable."1.88.0".default.override {
          extensions = [
            "rust-src"
            "clippy"
            "rustfmt"
          ];
        };

      in
      {
        defaultPackage = pkgs.callPackage ./package.nix { };

        devShell = pkgs.mkShell {
          nativeBuildInputs = [
            rustToolchain
          ];

          buildInputs =  [
            pkgs.openssl
            pkgs.pkg-config

            pkgs.rust-analyzer
            pkgs.jetbrains.rust-rover
          ];

          shellHook = ''
            mkdir -p ~/.rust-rover/toolchain

            ln -sfn ${rustToolchain}/lib ~/.rust-rover/toolchain
            ln -sfn ${rustToolchain}/bin ~/.rust-rover/toolchain

            export RUST_SRC_PATH="$HOME/.rust-rover/toolchain/lib/rustlib/src/rust/library"
          '';
        };
      }
    )
    // {
      overlays.default = (
        final: prev: {
          topcat = prev.callPackage ./package.nix { };
        }
      );
    };
}
