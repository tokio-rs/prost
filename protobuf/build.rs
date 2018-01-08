extern crate curl;
extern crate flate2;
extern crate tar;
extern crate tempdir;

use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use curl::easy::Easy;
use flate2::bufread::GzDecoder;
use tar::Archive;
use tempdir::TempDir;

const VERSION: &'static str = "3.5.1.1";

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    let protobuf_dir = out_dir.join(format!("protobuf-{}", VERSION));

    if !protobuf_dir.exists() {
        let tempdir = TempDir::new_in(&out_dir, "protobuf")
                              .expect("failed to create temporary directory");

        // Download the protobuf source tarball.
        download_protobuf(tempdir.path());
        let src_dir = tempdir.path().join(format!("protobuf-{}", VERSION));
        let prefix_dir = tempdir.path().join("prefix");
        fs::create_dir(&prefix_dir).expect("failed to create temporary build directory");

        // Build and install protoc, the protobuf libraries, and the conformance test runner.
        let rc = Command::new("./autogen.sh")
                        .current_dir(&src_dir)
                        .status()
                        .expect("failed to execute autogen.sh");
        assert!(rc.success(), "protobuf autogen.sh failed");

        let num_jobs = env::var("NUM_JOBS").expect("NUM_JOBS environment variable not set");

        let rc = Command::new("./configure")
                        .arg("--disable-shared")
                        .arg("--prefix").arg(&prefix_dir)
                        .current_dir(&src_dir)
                        .status()
                        .expect("failed to execute configure");
        assert!(rc.success(), "failed to configure protobuf");

        let rc = Command::new("make")
                        .arg("-j").arg(&num_jobs)
                        .arg("install")
                        .current_dir(&src_dir)
                        .status()
                        .expect("failed to execute make protobuf");
        assert!(rc.success(), "failed to make protobuf");

        let rc = Command::new("make")
                        .arg("-j").arg(&num_jobs)
                        .arg("install")
                        .current_dir(src_dir.join("conformance"))
                        .status()
                        .expect("failed to execute make conformance");
        assert!(rc.success(), "failed to make protobuf");

        // Copy .protos and data into the install directory.
        fs::create_dir(prefix_dir.join("share")).expect("failed to create share directory");
        fs::create_dir(prefix_dir.join("include").join("benchmarks"))
           .expect("failed to create benchmarks include directory");
        fs::create_dir(prefix_dir.join("include").join("conformance"))
           .expect("failed to create conformance include directory");

        for proto in &[
            "benchmarks/benchmark_messages_proto2.proto",
            "benchmarks/benchmark_messages_proto3.proto",
            "conformance/conformance.proto"
        ] {
            fs::rename(src_dir.join(proto), prefix_dir.join("include").join(proto))
               .expect(&format!("failed to move {}", proto));
        }
        for proto in &[
            "google/protobuf/test_messages_proto2.proto",
            "google/protobuf/test_messages_proto3.proto",
            "google/protobuf/unittest.proto",
            "google/protobuf/unittest_import.proto",
            "google/protobuf/unittest_import_public.proto"
        ] {
            fs::rename(src_dir.join("src").join(proto), prefix_dir.join("include").join(proto))
               .expect(&format!("failed to move {}", proto));
        }

        for dat in &["google_message1.dat", "google_message2.dat"] {
            fs::rename(src_dir.join("benchmarks").join(dat), prefix_dir.join("share").join(dat))
            .expect(&format!("failed to move {}", dat));
        }

        fs::rename(&prefix_dir, &protobuf_dir).expect("unable to move temporary directory");
    }

    // Emit an environment variable with the path to the conformance test runner
    // so that it can be used in the conformance tests.
    println!("cargo:rustc-env=PROTOBUF={}", protobuf_dir.display());
}

fn download_protobuf(out_dir: &Path) {
    let mut data = Vec::new();
    let mut handle = Easy::new();

    handle.url(&format!("https://github.com/google/protobuf/archive/v{}.tar.gz", VERSION))
          .expect("failed to configure protobuf tarball URL");
    handle.follow_location(true)
          .expect("failed to configure follow location");
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        }).expect("failed to write download data");
        transfer.perform().expect("failed to download protobuf");
    }

    Archive::new(GzDecoder::new(Cursor::new(data)))
            .unpack(out_dir).expect("failed to unpack protobuf tarball");
}
