//! Finds the appropriate `protoc` binary and Protobuf include directory for this host, and outputs
//! build directives so that the main `prost-build` crate can use them.
//!
//! The following locations are checked for `protoc` in decreasing priority:
//!
//!     1. The `PROTOC` environment variable.
//!     2. The vendored `protoc`.
//!     3. The `protoc` on the `PATH`.
//!
//! If no `protoc` binary is available in these locations, the build fails.
//!
//! The following locations are checked for the Protobuf include directory in decreasing priority:
//!
//!     1. The `PROTOC_INCLUDE` environment variable.
//!     2. The vendored Protobuf include directory.

use std::env;
use std::path::PathBuf;

use cfg_if::cfg_if;

/// Returns the path to the `protoc` pointed to by the `PROTOC` environment variable, if it is set.
fn env_protoc() -> Option<PathBuf> {
    let protoc = match env::var_os("PROTOC") {
        Some(path) => PathBuf::from(path),
        None => return None,
    };

    Some(protoc)
}

/// Returns the path to the vendored `protoc`, if it is available for the host platform.
fn vendored_protoc() -> Option<PathBuf> {
    cfg_if! {
        if #[cfg(feature = "protoc-vendored-bin")] {
            protoc_bin::protoc()
        } else if #[cfg(feature = "protoc-vendored-src")] {
            Some(protobuf_src::protoc())
        } else {
            None
        }
    }
}

/// Returns the path to the `protoc` included on the `PATH`, if it exists.
fn path_protoc() -> Option<PathBuf> {
    which::which("protoc").ok()
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

/// Returns the path to the vendored Protobuf include directory.
fn vendored_protoc_include() -> Option<PathBuf> {
    cfg_if! {
        if #[cfg(feature = "protoc-vendored-bin")] {
            Some(protoc_bin::include())
        } else if #[cfg(feature = "protoc-vendored-src")] {
            Some(protobuf_src::include())
        } else {
            None
        }
    }
}

fn main() {
    if cfg!(all(feature = "protoc-vendored-bin", feature = "protoc-vendored-src")) {
        panic!(
            "The `protoc-vendored-bin` and `protoc-vendored-src` features \
             cannot be enabled simultaneously",
        );
    }

    let protoc = env_protoc()
        .or_else(vendored_protoc)
        .or_else(path_protoc)
        .expect(
            "Failed to find the protoc binary. The PROTOC environment variable is not set, \
             there is no vendored protoc for this platform, and protoc is not in the PATH",
        );

    let protoc_include = env_protoc_include()
        .or_else(vendored_protoc_include)
        .expect(
            "Failed to find the protoc include path. The PROTOC_INCLUDE environment variable \
             is not set and neither `protoc-vendored` or `protoc-vendored-bin` was enabled",
        );

    println!("cargo:rustc-env=PROTOC={}", protoc.display());
    println!(
        "cargo:rustc-env=PROTOC_INCLUDE={}",
        protoc_include.display()
    );
    println!("cargo:rerun-if-env-changed=PROTOC");
    println!("cargo:rerun-if-env-changed=PROTOC_INCLUDE");
}
