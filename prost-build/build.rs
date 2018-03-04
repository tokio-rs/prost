use std::env;
use std::path::PathBuf;

fn main() {
    if env_contains_protoc() { return; }

    let protoc_bin_name = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86") => "protoc-linux-x86_32",
        ("linux", "x86_64") => "protoc-linux-x86_64",
        ("linux", "aarch64") => "protoc-linux-aarch_64",
        ("macos", "x86_64") => "protoc-osx-x86_64",
        ("windows", _) => "protoc-win32.exe",
        _ => panic!("no precompiled protoc binary for the current platform: {}-{}",
                    env::consts::OS, env::consts::ARCH),
    };

    let cwd = env::current_dir().unwrap();
    let protobuf = cwd.join("third-party/protobuf");
    let protoc = protobuf.join(protoc_bin_name);
    let protoc_include_dir = protobuf.join("include");

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

    println!("cargo:rustc-env=PROTOC={}", protoc.display());
    println!("cargo:rustc-env=PROTOC_INCLUDE={}", protoc_include.display());
    println!("cargo:rerun-if-env-changed=PROTOC");
    println!("cargo:rerun-if-env-changed=PROTOC_INCLUDE");
    true
}

