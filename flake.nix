{
  description = "Prost dependencies";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rustVersion = cargoToml.workspace.package.rust-version;
        default_pkgs = with pkgs; [
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
            rustpkgs = pkgs.rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            };
          in
          pkgs.mkShell {
            packages = [
              rustpkgs
            ]
            ++ default_pkgs;
          };
        devShells."rust_minimum_version" =
          let
            rustpkgs = pkgs.rust-bin.stable."${rustVersion}.0".default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            };
          in
          pkgs.mkShell {
            packages = [
              rustpkgs
            ]
            ++ default_pkgs;
          };
      }
    );
}
