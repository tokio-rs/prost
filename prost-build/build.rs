extern crate reqwest;
extern crate tempdir;
extern crate zip;

use std::env;
use std::fs;
use std::io::{
    self,
    Cursor,
};
use std::path::{
    Path,
    PathBuf,
};

use std::io::Read;
use tempdir::TempDir;
use zip::ZipArchive;

const VERSION: &'static str = "3.3.0";

fn main() {
    if env_contains_protoc() { return; }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable is invalid"));
    let protoc_dir = out_dir.join("protoc");

    // If the protoc directory doesn't already exist from a previous build, download protoc.
    if !protoc_dir.exists() {
        // The unzipping done by download_protoc is not atomic, so to avoid failing halfway through
        // and leaving trash state, download/unzip to a temporary directory, then rename.
        let tempdir = TempDir::new_in(&out_dir, "protoc").expect("failed to create temporary directory");
        download_protoc(tempdir.path());
        fs::rename(&tempdir.into_path(), &protoc_dir).expect("unable to move temporary directory");
    }
    let mut protoc = protoc_dir.join("bin");
    protoc.push("protoc");
    protoc.set_extension(env::consts::EXE_EXTENSION);

    println!("cargo:rustc-env=PROTOC={}", protoc.display());
    println!("cargo:rustc-env=PROTOC_INCLUDE={}", protoc_dir.join("include").display());
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
    true
}

/// Downloads and unpacks the protoc package for the current architecture to the target path.
/// Returns the paths to `protoc` and the include directory.
fn download_protoc(target: &Path) {
    let url = protoc_url();
    let mut data = Vec::new();

    let mut resp = reqwest::get(&url).expect("failed to download");
    if !resp.status().is_success() {
        panic!("failed to download, status code: {}", resp.status())
    }
    let _ = resp.read_to_end(&mut data).expect("failed to read data");

    let mut archive = ZipArchive::new(Cursor::new(data)).expect("failed to open zip archive");

    for i in 0..archive.len() {
        let mut src = archive.by_index(i).expect("failed to index into zip archive");

        let mut path = target.to_owned();
        path.push(src.name());

        if src.name().ends_with('/') {
            fs::create_dir(&path).unwrap();
        } else {
            let mut dest = &mut fs::File::create(&path).unwrap();
            io::copy(&mut src, &mut dest).unwrap();

            #[cfg(unix)]
            fn convert_permissions(mode: u32) -> Option<fs::Permissions> {
                use std::os::unix::fs::PermissionsExt;
                Some(fs::Permissions::from_mode(mode))
            }
            #[cfg(not(unix))]
            fn convert_permissions(_mode: u32) -> Option<fs::Permissions> {
                None
            }
            if let Some(permissions) = src.unix_mode().and_then(convert_permissions) {
                fs::set_permissions(&path, permissions).unwrap();
            }
        }
    }
}

fn protoc_url() -> String {
    let platform = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86")    => "linux-x86_32",
        ("linux", "x86_64") => "linux-x86_64",
        ("macos", "x86")    => "osx-x86_32",
        ("macos", "x86_64") => "osx-x86_64",
        ("windows", _)      => "win32",
        _ => panic!("no precompiled protoc binary for the current platform: {}-{}",
                    env::consts::OS, env::consts::ARCH),
    };
    format!("https://github.com/google/protobuf/releases/download/v{version}/protoc-{version}-{platform}.zip",
            version = VERSION,
            platform = platform)
}
