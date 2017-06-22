extern crate curl;
extern crate env_logger;
extern crate flate2;
extern crate num_cpus;
extern crate proto_build;
extern crate tar;

use std::env;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;

use curl::easy::Easy;
use flate2::bufread::GzDecoder;
use tar::Archive;

const VERSION: &'static str = "3.3.0";

fn main() {
    env_logger::init().unwrap();
    let target = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    let dir = target.join(format!("protobuf-{}", VERSION));
    let conformance_dir = dir.join("conformance");
    let conformance_bin = conformance_dir.join("conformance-test-runner");

    // Download the protobuf source tarball.
    if !dir.exists() {
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

        Archive::new(GzDecoder::new(Cursor::new(data)).expect("failed to create gzip decoder"))
                .unpack(target).expect("failed to unpack protobuf tarball");
    }

    if !dir.join("configure").exists() {
        let autogen_rc = Command::new("./autogen.sh")
                                 .current_dir(&dir)
                                 .status()
                                 .expect("failed to execute autogen.sh");
        assert!(autogen_rc.success(), "protobuf autogen.sh failed");
    }

    if !conformance_bin.exists() {
        let configure_rc = Command::new("./configure")
                                   .current_dir(&dir)
                                   .status()
                                   .expect("failed to execute configure");
        assert!(configure_rc.success(), "failed to configure protobuf");

        let make_rc = Command::new("make")
                              .arg("-j").arg(num_cpus::get().to_string())
                              .current_dir(&dir)
                              .status()
                              .expect("failed to execute make protobuf");
        assert!(make_rc.success(), "failed to make protobuf");

        let conformance_rc = Command::new("make")
                                     .arg("-j").arg(num_cpus::get().to_string())
                                     .current_dir(&conformance_dir)
                                     .status()
                                     .expect("failed to execute make conformance");
        assert!(conformance_rc.success(), "failed to make conformance");
    }


    // Emit an environment variable with the path to the conformance test runner
    // so that it can be used in the conformance tests.
    println!("cargo:rustc-env=CONFORMANCE_TEST_RUNNER={:?}", conformance_bin);

    proto_build::compile_protos(&[conformance_dir.join("conformance.proto")],
                                &[conformance_dir]).unwrap();
}
