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
            rustpkgs = fenix.packages.${system}.stable.defaultToolchain;
          in
          pkgs.mkShell {
            packages = [
              rustpkgs
            ] ++ default_pkgs;
          };
        devShells."rust_minimum_version" =
          let
            rust_manifest = {
              url = "https://static.rust-lang.org/dist/2023-08-03/channel-rust-1.71.1.toml";
              flake = false;
            };
            rustpkgs = (fenix.packages.${system}.fromManifestFile rust_manifest).defaultToolchain;
          in
          pkgs.mkShell {
            packages = [
              rustpkgs
            ] ++ default_pkgs;
          };
      }
    );
}
