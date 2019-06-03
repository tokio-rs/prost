use prost_build;

use tempfile;

use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use curl::easy::Easy;
use flate2::bufread::GzDecoder;
use tar::Archive;

const VERSION: &'static str = "3.8.0";

static TEST_PROTOS: &[&str] = &[
    "test_messages_proto2.proto",
    "test_messages_proto3.proto",
    "unittest.proto",
    "unittest_import.proto",
    "unittest_import_public.proto",
];

static DATASET_PROTOS: &[&str] = &[
    "google_message1/proto2/benchmark_message1_proto2.proto",
    "google_message1/proto3/benchmark_message1_proto3.proto",
    "google_message2/benchmark_message2.proto",
    "google_message3/benchmark_message3.proto",
    "google_message3/benchmark_message3_1.proto",
    "google_message3/benchmark_message3_2.proto",
    "google_message3/benchmark_message3_3.proto",
    "google_message3/benchmark_message3_4.proto",
    "google_message3/benchmark_message3_5.proto",
    "google_message3/benchmark_message3_6.proto",
    "google_message3/benchmark_message3_7.proto",
    "google_message3/benchmark_message3_8.proto",
    "google_message4/benchmark_message4.proto",
    "google_message4/benchmark_message4_1.proto",
    "google_message4/benchmark_message4_2.proto",
    "google_message4/benchmark_message4_3.proto",
];

fn main() {
    let out_dir =
        &PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    let protobuf_dir = &out_dir.join(format!("protobuf-{}", VERSION));

    if !protobuf_dir.exists() {
        let tempdir = tempfile::Builder::new()
            .prefix("protobuf")
            .tempdir_in(out_dir)
            .expect("failed to create temporary directory");

        let src_dir = &download_protobuf(tempdir.path());
        let prefix_dir = &src_dir.join("prefix");
        fs::create_dir(prefix_dir).expect("failed to create prefix directory");
        install_conformance_test_runner(src_dir, prefix_dir);
        install_protos(src_dir, prefix_dir);
        install_datasets(src_dir, prefix_dir);
        fs::rename(prefix_dir, protobuf_dir).expect("failed to move protobuf dir");
    }

    let include_dir = &protobuf_dir.join("include");
    let benchmarks_include_dir = &include_dir.join("benchmarks");
    let datasets_include_dir = &benchmarks_include_dir.join("datasets");
    let mut benchmark_protos = vec![benchmarks_include_dir.join("benchmarks.proto")];
    benchmark_protos.extend(
        DATASET_PROTOS
            .iter()
            .map(|proto| datasets_include_dir.join(proto)),
    );
    prost_build::compile_protos(&benchmark_protos, &[benchmarks_include_dir.to_path_buf()])
        .unwrap();

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
        .btree_map(&["."])
        .compile_protos(
            &[
                test_includes.join("test_messages_proto2.proto"),
                test_includes.join("test_messages_proto3.proto"),
                test_includes.join("unittest.proto"),
            ],
            &[include_dir.to_path_buf()],
        )
        .unwrap();

    // Emit an environment variable with the path to the build so that it can be located in the
    // main crate.
    println!("cargo:rustc-env=PROTOBUF={}", protobuf_dir.display());
}

fn download_tarball(url: &str, out_dir: &Path) {
    let mut data = Vec::new();
    let mut handle = Easy::new();

    handle.url(url).expect("failed to configure tarball URL");
    handle
        .follow_location(true)
        .expect("failed to configure follow location");
    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|new_data| {
                data.extend_from_slice(new_data);
                Ok(new_data.len())
            })
            .expect("failed to write download data");
        transfer.perform().expect("failed to download tarball");
    }

    Archive::new(GzDecoder::new(Cursor::new(data)))
        .unpack(out_dir)
        .expect("failed to unpack tarball");
}

/// Downloads and unpacks a Protobuf release tarball to the provided directory.
fn download_protobuf(out_dir: &Path) -> PathBuf {
    download_tarball(
        &format!(
            "https://github.com/google/protobuf/archive/v{}.tar.gz",
            VERSION
        ),
        out_dir,
    );
    out_dir.join(format!("protobuf-{}", VERSION))
}

fn install_conformance_test_runner(src_dir: &Path, prefix_dir: &Path) {
    #[cfg(not(windows))]
    {
        // Build and install protoc, the protobuf libraries, and the conformance test runner.
        let rc = Command::new("./autogen.sh")
            .current_dir(&src_dir)
            .status()
            .expect("failed to execute autogen.sh");
        assert!(rc.success(), "protobuf autogen.sh failed");

        let num_jobs = env::var("NUM_JOBS").expect("NUM_JOBS environment variable not set");

        let rc = Command::new("./configure")
            .arg("--disable-shared")
            .arg("--prefix")
            .arg(&prefix_dir)
            .current_dir(&src_dir)
            .status()
            .expect("failed to execute configure");
        assert!(rc.success(), "failed to configure protobuf");

        let rc = Command::new("make")
            .arg("-j")
            .arg(&num_jobs)
            .arg("install")
            .current_dir(&src_dir)
            .status()
            .expect("failed to execute make protobuf");
        assert!(rc.success(), "failed to make protobuf");

        let rc = Command::new("make")
            .arg("-j")
            .arg(&num_jobs)
            .arg("install")
            .current_dir(src_dir.join("conformance"))
            .status()
            .expect("failed to execute make conformance");
        assert!(rc.success(), "failed to make conformance");
    }
}

fn install_protos(src_dir: &Path, prefix_dir: &Path) {
    let include_dir = prefix_dir.join("include");

    // Move test protos to the prefix directory.
    let test_include_dir = &include_dir.join("google").join("protobuf");
    fs::create_dir_all(test_include_dir).expect("failed to create test include directory");
    for proto in TEST_PROTOS {
        fs::rename(
            src_dir
                .join("src")
                .join("google")
                .join("protobuf")
                .join(proto),
            test_include_dir.join(proto),
        )
        .expect(&format!("failed to move {}", proto));
    }

    // Move conformance.proto to the install directory.
    let conformance_include_dir = &include_dir.join("conformance");
    fs::create_dir(conformance_include_dir)
        .expect("failed to create conformance include directory");
    fs::rename(
        src_dir.join("conformance").join("conformance.proto"),
        conformance_include_dir.join("conformance.proto"),
    )
    .expect("failed to move conformance.proto");

    // Move the benchmark datasets to the install directory.
    let benchmarks_src_dir = &src_dir.join("benchmarks");
    let benchmarks_include_dir = &include_dir.join("benchmarks");
    let datasets_src_dir = &benchmarks_src_dir.join("datasets");
    let datasets_include_dir = &benchmarks_include_dir.join("datasets");
    fs::create_dir(benchmarks_include_dir).expect("failed to create benchmarks include directory");
    fs::rename(
        benchmarks_src_dir.join("benchmarks.proto"),
        benchmarks_include_dir.join("benchmarks.proto"),
    )
    .expect("failed to move benchmarks.proto");
    for proto in DATASET_PROTOS.iter().map(Path::new) {
        let dir = &datasets_include_dir.join(proto.parent().unwrap());
        fs::create_dir_all(dir).expect(&format!("unable to create directory {}", dir.display()));
        fs::rename(
            datasets_src_dir.join(proto),
            datasets_include_dir.join(proto),
        )
        .expect(&format!("failed to move {}", proto.display()));
    }
}

fn install_datasets(src_dir: &Path, prefix_dir: &Path) {
    let share_dir = &prefix_dir.join("share");
    fs::create_dir(share_dir).expect("failed to create share directory");
    for dataset in &[
        Path::new("google_message1")
            .join("proto2")
            .join("dataset.google_message1_proto2.pb"),
        Path::new("google_message1")
            .join("proto3")
            .join("dataset.google_message1_proto3.pb"),
        Path::new("google_message2").join("dataset.google_message2.pb"),
    ] {
        fs::rename(
            src_dir.join("benchmarks").join("datasets").join(dataset),
            share_dir.join(dataset.file_name().unwrap()),
        )
        .expect(&format!("failed to move {}", dataset.display()));
    }

    download_tarball(
        "https://storage.googleapis.com/protobuf_opensource_benchmark_data/datasets.tar.gz",
        share_dir,
    );
}
