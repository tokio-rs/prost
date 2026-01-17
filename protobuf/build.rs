use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

// Protobuf version to fetch
const PROTOBUF_VERSION: &str = "25.8";
const PROTOBUF_TAG: &str = "v25.8";

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").context("OUT_DIR not set")?);
    let protobuf_dir = out_dir.join(format!("protobuf-{PROTOBUF_VERSION}"));

    if !protobuf_dir.exists() {
        build_protobuf(&out_dir, &protobuf_dir)?;
    }

    compile_proto_files(&out_dir, &protobuf_dir)?;

    println!("cargo:rustc-env=PROTOBUF={}", protobuf_dir.display());
    Ok(())
}

fn build_protobuf(out_dir: &Path, protobuf_dir: &Path) -> Result<()> {
    let build_dir = out_dir.join(format!("build-protobuf-{PROTOBUF_VERSION}"));
    fs::create_dir_all(&build_dir).context("failed to create build directory")?;

    let tempdir = tempfile::Builder::new()
        .prefix("protobuf")
        .tempdir_in(out_dir)
        .context("failed to create temporary directory")?;

    let prefix_dir = tempdir.path().join("prefix");
    fs::create_dir(&prefix_dir).context("failed to create prefix directory")?;

    write_cmake_file(&build_dir)?;
    build_with_cmake(&build_dir, &prefix_dir)?;

    fs::rename(&prefix_dir, protobuf_dir).context("failed to move protobuf dir")?;
    Ok(())
}

fn write_cmake_file(build_dir: &Path) -> Result<()> {
    let system_processor = match () {
        _ if cfg!(target_arch = "aarch64") => "aarch64",
        _ if cfg!(target_arch = "x86_64") => "x86_64",
        _ => "unknown",
    };

    let build_conformance = if cfg!(windows) { "OFF" } else { "ON" };

    let cmake_content = format!(
        r#"cmake_minimum_required(VERSION 3.14)

# Set processor type BEFORE project() so CMake detection works correctly
set(CMAKE_SYSTEM_PROCESSOR {system_processor})

project(protobuf-fetcher C CXX)

include(FetchContent)

FetchContent_Declare(
  protobuf
  GIT_REPOSITORY https://github.com/protocolbuffers/protobuf.git
  GIT_TAG {tag}
  GIT_SHALLOW TRUE
)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(ABSL_PROPAGATE_CXX_STD ON)
set(ABSL_USE_EXTERNAL_GOOGLETEST ON)
set(ABSL_BUILD_TESTING OFF)
set(ABSL_ENABLE_INSTALL ON)
set(protobuf_BUILD_CONFORMANCE {conformance})
set(protobuf_BUILD_TESTS OFF)
set(protobuf_ABSL_PROVIDER "module")

# On macOS with Nix, abseil's multi-arch support conflicts with Nix's --target flags.
# Only apply workaround if we detect Nix environment (CMAKE_C_FLAGS contains --target).
if(APPLE AND CMAKE_OSX_ARCHITECTURES STREQUAL "arm64")
  # Check if CMAKE_C_FLAGS suggests we're in Nix (contains --target)
  string(FIND "${{CMAKE_C_FLAGS}}" "--target" NIX_DETECTED)
  if(NOT NIX_DETECTED EQUAL -1)
    message(STATUS "Detected Nix environment, applying abseil workaround")
    set(_SAVED_APPLE "${{APPLE}}")
    set(APPLE FALSE CACHE BOOL "" FORCE)
  endif()
endif()

FetchContent_MakeAvailable(protobuf)

# Restore APPLE flag after configuration
if(_SAVED_APPLE)
  set(APPLE "${{_SAVED_APPLE}}" CACHE BOOL "" FORCE)
endif()
"#,
        system_processor = system_processor,
        tag = PROTOBUF_TAG,
        conformance = build_conformance
    );

    fs::write(build_dir.join("CMakeLists.txt"), cmake_content)
        .context("failed to write CMakeLists.txt")
}

fn build_with_cmake(build_dir: &Path, prefix_dir: &Path) -> Result<()> {
    let mut config = cmake::Config::new(build_dir);
    config
        .define("CMAKE_INSTALL_PREFIX", prefix_dir)
        .define("CMAKE_CXX_STANDARD", "14")
        .define("ABSL_PROPAGATE_CXX_STD", "ON")
        .out_dir(build_dir);

    if cfg!(target_arch = "aarch64") {
        config
            .define("ABSL_USE_EXTERNAL_GOOGLETEST", "ON")
            .define("ABSL_BUILD_TESTING", "OFF");
    }

    // On Windows, reduce parallel build jobs to avoid resource exhaustion
    if cfg!(windows) {
        config.build_arg("-j2");
    }

    config.build();

    // Copy conformance-test-runner if it was built (non-Windows only)
    if !cfg!(windows) {
        let conformance_runner = build_dir
            .join("build")
            .join("_deps")
            .join("protobuf-build")
            .join("conformance_test_runner");

        if conformance_runner.exists() {
            fs::copy(
                &conformance_runner,
                prefix_dir.join("bin").join("conformance-test-runner"),
            )
            .context("failed to copy conformance-test-runner")?;
        }
    }

    Ok(())
}

fn compile_proto_files(out_dir: &Path, protobuf_dir: &Path) -> Result<()> {
    let protoc_name = if cfg!(windows) {
        "protoc.exe"
    } else {
        "protoc"
    };
    let protoc_executable = protobuf_dir.join("bin").join(protoc_name);

    if !protoc_executable.exists() {
        anyhow::bail!(
            "protoc not found at {}. Build may have failed.",
            protoc_executable.display()
        );
    }

    // On macOS, set DYLD_LIBRARY_PATH so protoc can find shared libraries
    if cfg!(target_os = "macos") {
        let lib_dir = protobuf_dir.join("lib");
        let current = env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
        let new_path = if current.is_empty() {
            lib_dir.display().to_string()
        } else {
            format!("{}:{}", lib_dir.display(), current)
        };
        // SAFETY: We're in a build script, setting DYLD_LIBRARY_PATH for child processes
        // (protoc) to find shared libraries. This is the intended use case.
        unsafe {
            env::set_var("DYLD_LIBRARY_PATH", new_path);
        }
    }

    let protobuf_src = out_dir
        .join(format!("build-protobuf-{PROTOBUF_VERSION}"))
        .join("build")
        .join("_deps")
        .join("protobuf-src");

    if !protobuf_src.exists() {
        anyhow::bail!(
            "Protobuf source not found at {}. CMake FetchContent may have failed.",
            protobuf_src.display()
        );
    }

    // Compile conformance.proto if it exists
    let conformance_dir = protobuf_src.join("conformance");
    if conformance_dir.exists() {
        prost_build::Config::new()
            .protoc_executable(&protoc_executable)
            .compile_protos(
                &[conformance_dir.join("conformance.proto")],
                &[&conformance_dir],
            )
            .context("failed to compile conformance.proto")?;
    }

    // Compile test proto files with BTreeMap for consistent encoding
    let proto_dir = protobuf_src.join("src");
    prost_build::Config::new()
        .protoc_executable(&protoc_executable)
        .btree_map(["."])
        .compile_protos(
            &[
                proto_dir.join("google/protobuf/test_messages_proto2.proto"),
                proto_dir.join("google/protobuf/test_messages_proto3.proto"),
                proto_dir.join("google/protobuf/unittest.proto"),
            ],
            &[&proto_dir],
        )
        .context("failed to compile test protos")?;

    Ok(())
}
