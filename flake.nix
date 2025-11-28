{
  description = "Prost dependencies";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        default_pkgs = with pkgs; [
          protobuf
          cmake
          pkg-config
          protobuf
          curl
          ninja
        ];
      in
      {
        devShells.default =
          let
            rustpkgs = fenix.packages.${system}.stable.completeToolchain;
          in
          pkgs.mkShell {
            packages = [
              rustpkgs
            ] ++ default_pkgs;
          };
        devShells."rust_minimum_version" =
          let
            rustpkgs = (fenix.packages.${system}.fromToolchainName {
              name = "1.82";
              sha256 = "sha256-yMuSb5eQPO/bHv+Bcf/US8LVMbf/G/0MSfiPwBhiPpk=";
            }).completeToolchain;
          in
          pkgs.mkShell {
            packages = [
              rustpkgs
            ] ++ default_pkgs;
          };
      }
    );
}
