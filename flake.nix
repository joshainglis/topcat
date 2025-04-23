{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
          overlays = [
            naersk.outputs.overlay
          ];
        };
      in
      {
        defaultPackage = pkgs.naersk.buildPackage { src = ./.; };

        overlays.default = (final: prev: {
          topcat = prev.naersk.buildPackage.buildPackage { src = ./.; };
        });

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ];
        };
      }
    );
}
