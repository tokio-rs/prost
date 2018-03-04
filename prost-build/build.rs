extern crate fs_extra;

use fs_extra::dir;

use std::env;
use std::fs;
use std::path::{
    PathBuf,
};

fn main() {
    if env_contains_protoc() { return; }

    // Recursively copy all the protobuf files to `OUT_DIR` so that they are
    // available for use at runtime.

    let src_dir = PathBuf::from("../third-party/protobuf");
    static INCLUDE: &str = "include";

    let protoc_bin_name = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86") => "protoc-linux-x86_32",
        ("linux", "x86_64") => "protoc-linux-x86_64",
        ("linux", "aarch64") => "protoc-linux-aarch_64",
        ("macos", "x86_64") => "protoc-osx-x86_64",
        ("windows", _) => "protoc-win32.exe",
        _ => panic!("no precompiled protoc binary for the current platform: {}-{}",
                    env::consts::OS, env::consts::ARCH),
    };

    let dest_dir = {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR")
            .expect("OUT_DIR environment variable is invalid"));
        out_dir.join("protobuf")
    };

    let protoc = dest_dir.join(protoc_bin_name);
    let protoc_include_dir = dest_dir.join(INCLUDE);

    fs::create_dir_all(&protoc_include_dir).unwrap();

    fs::copy(src_dir.join(protoc_bin_name), &protoc).unwrap();

    let options = {
        let mut options = dir::CopyOptions::new();
        options.overwrite = true;
        options
    };
    dir::copy(src_dir.join(INCLUDE), &dest_dir, &options).unwrap();

    println!("cargo:rustc-env=PROTOC={}", protoc.display());
    println!("cargo:rustc-env=PROTOC_INCLUDE={}", protoc_include_dir.display());
    println!("cargo:rerun-if-env-changed=PROTOC");
    println!("cargo:rerun-if-env-changed=PROTOC_INCLUDE");
}

/// Returns `true` if the environment already contains the `PROTOC` and `PROTOC_INCLUDE` variables.
fn env_contains_protoc() -> bool {
    let protoc = match env::var("PROTOC") {
        Ok(val) => PathBuf::from(val),
        Err(env::VarError::NotPresent) => return false,
        Err(env::VarError::NotUnicode(..)) => panic!("PROTOC environment variable is not valid UTF-8"),
    };

    let protoc_include = match env::var("PROTOC_INCLUDE") {
        Ok(val) => PathBuf::from(val),
        Err(env::VarError::NotPresent) => panic!("PROTOC_INCLUDE environment variable not set (PROTOC is set)"),
        Err(env::VarError::NotUnicode(..)) => panic!("PROTOC_INCLUDE environment variable is not valid UTF-8"),
    };

    if !protoc.exists() {
        panic!("PROTOC environment variable points to non-existent file ({:?})", protoc);
    }
    if !protoc_include.exists() {
        panic!("PROTOC_INCLUDE environment variable points to non-existent directory ({:?})",
               protoc_include);
    }

    // Even if PROTOC and PROTOC_INCLUDE are set in the environment, still 
    println!("cargo:rustc-env=PROTOC={}", protoc.display());
    println!("cargo:rustc-env=PROTOC_INCLUDE={}", protoc_include.display());
    println!("cargo:rerun-if-env-changed=PROTOC");
    println!("cargo:rerun-if-env-changed=PROTOC_INCLUDE");
    true
}

