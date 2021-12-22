//! `protoc-bin` vendors `protoc` binaries for common architectures.
//!
//! It is intended for use as a build dependency for  libraries like
//! [`prost-build`](https://docs.rs/prost_build).
//!
//! Binaries are currently provided for the following platforms:
//!
//!   * Linux x86
//!   * Linux x86_64
//!   * Linux aarch64
//!   * macOS x86_64
//!   * macOS aarch64
//!   * Windows 32-bit
//!

use std::env;
use std::path::PathBuf;

/// Returns the path to the location of the vendored Protobuf artifacts.
fn vendored_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("third-party")
        .join("protobuf")
}

/// We can only use a vendored protoc if the interpreter necessary to load the binary is available.
///
/// The interpreter is specific to the binary and can be queried via e.g. `patchelf
/// --print-interpreter`, or via readelf, or similar.
fn is_interpreter(path: &'static str) -> bool {
    // Here we'd check for it being executable and other things, but for now it being present is
    // probably good enough.
    std::fs::metadata(path).is_ok()
}

/// Returns the path to the vendored `protoc`, if it is available for the host
/// platform.
pub fn protoc() -> Option<PathBuf> {
    let protoc_bin_name = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86") if is_interpreter("/lib/ld-linux.so.2") => "protoc-linux-x86_32",
        ("linux", "x86_64") if is_interpreter("/lib64/ld-linux-x86-64.so.2") => {
            "protoc-linux-x86_64"
        }
        ("linux", "aarch64") if is_interpreter("/lib/ld-linux-aarch64.so.1") => {
            "protoc-linux-aarch_64"
        }
        ("macos", "x86_64") => "protoc-osx-x86_64",
        ("macos", "aarch64") => "protoc-osx-aarch64",
        ("windows", _) => "protoc-win32.exe",
        _ => return None,
    };

    Some(vendored_path().join(protoc_bin_name))
}

/// Returns the path to the vendored Protobuf include directory.
pub fn include() -> PathBuf {
    vendored_path().join("include")
}
