use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{ensure, Context, Result};

static TEST_PROTOS: &[&str] = &[
    "test_messages_proto2.proto",
    "test_messages_proto3.proto",
    "unittest.proto",
    "unittest_import.proto",
    "unittest_import_public.proto",
];

fn main() -> Result<()> {
    let out_dir =
        &PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));

    let src_dir = PathBuf::from("../third_party/protobuf");
    if !src_dir.join("cmake").exists() {
        anyhow::bail!(
            "protobuf sources are not checked out; Try `git submodule update --init --recursive`"
        )
    }

    let version = git_describe(&src_dir)?;
    let protobuf_dir = &out_dir.join(format!("protobuf-{}", version));

    if !protobuf_dir.exists() {
        apply_patches(&src_dir)?;
        let tempdir = tempfile::Builder::new()
            .prefix("protobuf")
            .tempdir_in(out_dir)
            .expect("failed to create temporary directory");

        let prefix_dir = &tempdir.path().join("prefix");
        fs::create_dir(prefix_dir).expect("failed to create prefix directory");
        install_conformance_test_runner(&src_dir, prefix_dir)?;
        install_protos(&src_dir, prefix_dir)?;
        fs::rename(prefix_dir, protobuf_dir).context("failed to move protobuf dir")?;
    }

    let include_dir = &protobuf_dir.join("include");

    let conformance_include_dir = include_dir.join("conformance");
    prost_build::compile_protos(
        &[conformance_include_dir.join("conformance.proto")],
        &[conformance_include_dir],
    )
    .unwrap();

    let test_includes = &include_dir.join("google").join("protobuf");

    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    prost_build::Config::new()
        .btree_map(["."])
        .compile_protos(
            &[
                test_includes.join("test_messages_proto2.proto"),
                test_includes.join("test_messages_proto3.proto"),
                test_includes.join("unittest.proto"),
            ],
            &[include_dir],
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

#[cfg(windows)]
fn install_conformance_test_runner(_: &Path, _: &Path) -> Result<()> {
    // The conformance test runner does not support Windows [1].
    // [1]: https://github.com/protocolbuffers/protobuf/tree/master/conformance#portability
    Ok(())
}

#[cfg(not(windows))]
fn install_conformance_test_runner(src_dir: &Path, prefix_dir: &Path) -> Result<()> {
    // Build and install protoc, the protobuf libraries, and the conformance test runner.
    let rc = Command::new("cmake")
        .arg("-GNinja")
        .arg("cmake/")
        .arg("-DCMAKE_BUILD_TYPE=DEBUG")
        .arg(&format!("-DCMAKE_INSTALL_PREFIX={}", prefix_dir.display()))
        .arg("-Dprotobuf_BUILD_CONFORMANCE=ON")
        .arg("-Dprotobuf_BUILD_TESTS=OFF")
        .current_dir(src_dir)
        .status()
        .context("failed to execute CMake")?;
    assert!(rc.success(), "protobuf CMake failed");

    let num_jobs = env::var("NUM_JOBS").context("NUM_JOBS environment variable not set")?;

    let rc = Command::new("ninja")
        .arg("-j")
        .arg(&num_jobs)
        .arg("install")
        .current_dir(src_dir)
        .status()
        .context("failed to execute ninja protobuf")?;
    ensure!(rc.success(), "failed to make protobuf");

    // Install the conformance-test-runner binary, since it isn't done automatically.
    fs::copy(
        src_dir.join("conformance_test_runner"),
        prefix_dir.join("bin").join("conformance-test-runner"),
    )
    .context("failed to move conformance-test-runner")?;

    Ok(())
}

fn install_protos(src_dir: &Path, prefix_dir: &Path) -> Result<()> {
    let include_dir = prefix_dir.join("include");

    // Move test protos to the prefix directory.
    let test_include_dir = &include_dir.join("google").join("protobuf");
    fs::create_dir_all(test_include_dir).expect("failed to create test include directory");
    for proto in TEST_PROTOS {
        fs::copy(
            src_dir
                .join("src")
                .join("google")
                .join("protobuf")
                .join(proto),
            test_include_dir.join(proto),
        )
        .with_context(|| format!("failed to move {}", proto))?;
    }

    // Move conformance.proto to the install directory.
    let conformance_include_dir = &include_dir.join("conformance");
    fs::create_dir(conformance_include_dir)
        .expect("failed to create conformance include directory");
    fs::copy(
        src_dir.join("conformance").join("conformance.proto"),
        conformance_include_dir.join("conformance.proto"),
    )
    .expect("failed to move conformance.proto");

    Ok(())
}
