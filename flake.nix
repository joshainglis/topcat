{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = inputs@{ self, flake-utils, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = (import nixpkgs) {
            inherit system;
            overlays = [
              inputs.naersk.outputs.overlay
            ];
          };
        in
        {
          defaultPackage = pkgs.naersk.buildPackage { src = ./.; };

          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [ rustc cargo ];
          };
        }
      ) //
    {
      overlays.default = (final: prev: {
        naersk = prev.callPackage inputs.naersk { };
        topcat = final.naersk.buildPackage { src = ./.; };
      });
    };
}
