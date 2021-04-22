let
  oxalica_overlay = import (builtins.fetchTarball
    "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  nixpkgs = import <nixpkgs> { overlays = [ oxalica_overlay ]; };
in with nixpkgs;
pkgs.mkShell {
    # nativeBuildInputs is usually what you want -- plugins you need to run
    nativeBuildInputs = [
        firefox
	    jetbrains.clion
	    openssl
	    cmake
	    ninja
        gcc
        rustup
        rust-bin.stable.latest.rust
        protobuf
	    buf
        rustPackages.clippy
        pkg-config
    ];
}
