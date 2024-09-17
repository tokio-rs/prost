use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{ensure, Context, Result};

fn main() -> Result<()> {
    let out_dir =
        &PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));

    let src_dir = std::path::absolute(PathBuf::from("../third_party/protobuf"))?;
    if !src_dir.join("cmake").exists() {
        anyhow::bail!(
            "protobuf sources are not checked out; Try `git submodule update --init --recursive`"
        )
    }

    let version = git_describe(&src_dir)?;
    let protobuf_dir = &out_dir.join(format!("protobuf-{}", version));

    if !protobuf_dir.exists() {
        apply_patches(&src_dir)?;

        let build_dir = &out_dir.join(format!("build-protobuf-{}", version));
        fs::create_dir_all(build_dir).expect("failed to create build directory");

        let tempdir = tempfile::Builder::new()
            .prefix("protobuf")
            .tempdir_in(out_dir)
            .expect("failed to create temporary directory");

        let prefix_dir = &tempdir.path().join("prefix");
        fs::create_dir(prefix_dir).expect("failed to create prefix directory");
        install_protoc_and_conformance_test_runner(&src_dir, build_dir, prefix_dir)?;
        fs::rename(prefix_dir, protobuf_dir).context("failed to move protobuf dir")?;
    }

    let protoc_executable = protobuf_dir.join("bin").join("protoc");

    let conformance_proto_dir = src_dir.join("conformance");
    prost_build::Config::new()
        .protoc_executable(&protoc_executable)
        .compile_protos(
            &[conformance_proto_dir.join("conformance.proto")],
            &[conformance_proto_dir],
        )
        .unwrap();

    let proto_dir = src_dir.join("src");

    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    prost_build::Config::new()
        .protoc_executable(&protoc_executable)
        .btree_map(["."])
        .compile_protos(
            &[
                proto_dir.join("google/protobuf/test_messages_proto2.proto"),
                proto_dir.join("google/protobuf/test_messages_proto3.proto"),
                proto_dir.join("google/protobuf/unittest.proto"),
            ],
            &[proto_dir],
        )
        .unwrap();

    // Emit an environment variable with the path to the build so that it can be located in the
    // main crate.
    println!("cargo:rustc-env=PROTOBUF={}", protobuf_dir.display());
    Ok(())
}

fn git_describe(src_dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("describe")
        .arg("--tags")
        .arg("--always")
        .current_dir(src_dir)
        .output()
        .context("Unable to describe protobuf git repo")?;
    if !output.status.success() {
        anyhow::bail!(
            "Unable to describe protobuf git repo: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

/// Apply patches to the protobuf source directory
fn apply_patches(src_dir: &Path) -> Result<()> {
    let mut patch_src = env::current_dir().context("failed to get current working directory")?;
    patch_src.push("src");
    patch_src.push("fix-conformance_test_runner-cmake-build.patch");

    let rc = Command::new("patch")
        .arg("-f")
        .arg("-p1")
        .arg("-i")
        .arg(patch_src)
        .current_dir(src_dir)
        .status()
        .context("failed to apply patch")?;
    // exit code: 0 means success; 1 means already applied
    ensure!(rc.code().unwrap() <= 1, "protobuf patch failed");

    Ok(())
}

fn install_protoc_and_conformance_test_runner(
    src_dir: &Path,
    build_dir: &Path,
    prefix_dir: &Path,
) -> Result<()> {
    // The protobuf conformance test runner does not support Windows [1].
    // [1]: https://github.com/protocolbuffers/protobuf/tree/master/conformance#portability
    let build_conformance = !cfg!(windows);

    // Build and install protoc, the protobuf libraries, and the conformance test runner.
    cmake::Config::new(src_dir.join("cmake"))
        .define("CMAKE_INSTALL_PREFIX", prefix_dir)
        .define(
            "protobuf_BUILD_CONFORMANCE",
            if build_conformance { "ON" } else { "OFF" },
        )
        .define("protobuf_BUILD_TESTS", "OFF")
        .out_dir(build_dir)
        .build();

    if build_conformance {
        // Install the conformance-test-runner binary, since it isn't done automatically.
        fs::copy(
            build_dir.join("build").join("conformance_test_runner"),
            prefix_dir.join("bin").join("conformance-test-runner"),
        )
        .context("failed to copy conformance-test-runner")?;
    }

    Ok(())
}
