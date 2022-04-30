//! Finds the appropriate `protoc` binary and Protobuf include directory for this host, and outputs
//! build directives so that the main `prost-build` crate can use them.
//!
//! This build script attempts to find `protoc` in a few ways:
//!
//!     1. If `PROTOC_NO_VENDOR` is enabled, it will check the `PROTOC` environment variable
//!         then check the `PATH` for a `protoc` or `protoc.exe`.
//!     2. If the `vendored` feature flag is enabled or `protoc` can't be found via the environment
//!         variable or in the `PATH` then `prost-build` will attempt to build `protoc` from the
//!         bundled source code.
//!     3. Otherwise, it will attempt to execute from the `PATH` and fail if it does not exist.
//!
//! The following locations are checked for the Protobuf include directory in decreasing priority:
//!
//!     1. The `PROTOC_INCLUDE` environment variable.
//!     2. The bundled Protobuf include directory.
//!

use cfg_if::cfg_if;
use std::env;
use std::path::PathBuf;
use which::which;

/// Returns the path to the location of the bundled Protobuf artifacts.
fn bundle_path() -> PathBuf {
    env::current_dir().unwrap().join("third-party")
}

/// Returns the path to the Protobuf include directory pointed to by the `PROTOC_INCLUDE`
/// environment variable, if it is set.
fn env_protoc_include() -> Option<PathBuf> {
    let protoc_include = match env::var_os("PROTOC_INCLUDE") {
        Some(path) => PathBuf::from(path),
        None => return None,
    };

    if !protoc_include.exists() {
        panic!(
            "PROTOC_INCLUDE environment variable points to non-existent directory ({:?})",
            protoc_include
        );
    }
    if !protoc_include.is_dir() {
        panic!(
            "PROTOC_INCLUDE environment variable points to a non-directory file ({:?})",
            protoc_include
        );
    }

    Some(protoc_include)
}

/// Returns the path to the bundled Protobuf include directory.
fn bundled_protoc_include() -> PathBuf {
    bundle_path().join("include")
}

/// Check for `protoc` via the `PROTOC` env var or in the `PATH`.
fn path_protoc() -> Option<PathBuf> {
    env::var_os("PROTOC")
        .map(PathBuf::from)
        .or_else(|| which("protoc").ok())
}

/// Returns true if the vendored flag is enabled.
fn vendored() -> bool {
    cfg_if! {
        if #[cfg(feature = "vendored")] {
            true
        } else {
            false
        }
    }
}

/// Compile `protoc` via `cmake`.
fn compile() -> Option<PathBuf> {
    let protobuf_src = bundle_path().join("protobuf").join("cmake");

    println!("cargo:rerun-if-changed={}", protobuf_src.display());

    let dst = cmake::Config::new(protobuf_src)
        .define("protobuf_BUILD_TESTS", "OFF")
        .build();

    Some(dst.join("bin").join("protoc"))
}

/// Try to find a `protoc` through a few methods.
///
/// Check module docs for more info.
fn protoc() -> Option<PathBuf> {
    if env::var_os("PROTOC_NO_VENDOR").is_some() {
        path_protoc()
    } else if vendored() {
        compile()
    } else {
        path_protoc().or_else(compile)
    }
}

fn main() {
    let protoc = protoc().expect(
        "Failed to find or build the protoc binary. The PROTOC environment \
    is not set, `protoc` is not in PATH or you are missing the requirements to compile protobuf \
    from source. \n \
    Check out the `prost-build` README for instructions on the requirements: \
    https://github.com/tokio-rs/prost#generated-code",
    );

    let protoc_include = env_protoc_include().unwrap_or_else(bundled_protoc_include);

    println!("cargo:rustc-env=PROTOC={}", protoc.display());
    println!(
        "cargo:rustc-env=PROTOC_INCLUDE={}",
        protoc_include.display()
    );
    println!("cargo:rerun-if-env-changed=PROTOC");
    println!("cargo:rerun-if-env-changed=PROTOC_INCLUDE");
}
